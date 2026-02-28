use macroquad::prelude::*;
use std::thread::{self, JoinHandle};

mod evolution;
mod game;
mod genome;
mod simulation;

use evolution::*;
use game::*;
use genome::*;

const END_DELAY: f32 = 2.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Evolved Spaceship Duel".to_string(),
        window_width: ARENA_WIDTH as i32,
        window_height: ARENA_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

/// Spawn evolution (evolve + evaluate) on a background thread.
/// Returns a join handle that yields the updated population and top two genomes.
fn spawn_evolution(mut pop: Population) -> JoinHandle<(Population, Genome, Genome)> {
    thread::spawn(move || {
        let mut rng = ::rand::thread_rng();
        pop.evolve(&mut rng);
        pop.evaluate(&mut rng);
        let (g1, g2) = pop.get_top_two();
        (pop, g1, g2)
    })
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut rng = ::rand::thread_rng();

    // Initialize population and run first evaluation synchronously
    let mut pop = Population::new(&mut rng);
    pop.evaluate(&mut rng);
    let (g1, g2) = pop.get_top_two();

    let mut current_gen = pop.generation;
    let mut current_best = pop.best_fitness;
    println!("Generation {} | Best fitness: {:.1}", current_gen, current_best);

    // Start first background evolution
    let mut evo_handle: Option<JoinHandle<(Population, Genome, Genome)>> =
        Some(spawn_evolution(pop));

    // Showcase state
    let mut showcase_genomes = [g1, g2];
    let mut match_state = GameState::new_random(&mut rng);
    let mut end_timer = END_DELAY;

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        if !match_state.match_over {
            // Step the showcase match
            let inputs0 = Genome::get_inputs(&match_state, 0);
            let inputs1 = Genome::get_inputs(&match_state, 1);
            let actions0 = showcase_genomes[0].evaluate(&inputs0);
            let actions1 = showcase_genomes[1].evaluate(&inputs1);
            match_state.update(dt, &[actions0, actions1]);
        } else {
            end_timer -= dt;
            match_state.time += dt;

            if end_timer <= 0.0 {
                // Check if background evolution has completed
                let evo_done = evo_handle
                    .as_ref()
                    .map_or(false, |h| h.is_finished());

                if evo_done {
                    let (new_pop, g1, g2) = evo_handle.take().unwrap().join().unwrap();
                    current_gen = new_pop.generation;
                    current_best = new_pop.best_fitness;
                    showcase_genomes = [g1, g2];
                    println!(
                        "Generation {} | Best fitness: {:.1}",
                        current_gen, current_best
                    );

                    // Start next background evolution
                    evo_handle = Some(spawn_evolution(new_pop));
                }

                // Start a new showcase match (with current or updated genomes)
                match_state = GameState::new_random(&mut rng);
                end_timer = END_DELAY;
            }
        }

        // Render
        clear_background(BLACK);
        render_arena();
        render_projectiles(&match_state.projectiles);
        render_ship(&match_state.ships[0], Color::new(0.0, 1.0, 0.4, 1.0));
        render_ship(&match_state.ships[1], Color::new(0.4, 0.6, 1.0, 1.0));
        render_hud(&match_state, current_gen, current_best);

        if match_state.match_over {
            render_match_result(&match_state);
        }

        next_frame().await;
    }
}

fn render_arena() {
    let border_color = Color::new(0.15, 0.15, 0.25, 1.0);
    let t = 1.0;
    draw_line(0.0, 0.0, ARENA_WIDTH, 0.0, t, border_color);
    draw_line(ARENA_WIDTH, 0.0, ARENA_WIDTH, ARENA_HEIGHT, t, border_color);
    draw_line(ARENA_WIDTH, ARENA_HEIGHT, 0.0, ARENA_HEIGHT, t, border_color);
    draw_line(0.0, ARENA_HEIGHT, 0.0, 0.0, t, border_color);
}

fn render_ship(ship: &Ship, color: Color) {
    if !ship.alive {
        render_explosion(ship.x, ship.y, color);
        return;
    }

    let cos = ship.rotation.cos();
    let sin = ship.rotation.sin();

    // Triangle vertices (nose forward)
    let nose = (ship.x + cos * SHIP_RADIUS, ship.y + sin * SHIP_RADIUS);
    let left = (
        ship.x + (-cos * 0.7 - sin * 0.7) * SHIP_RADIUS,
        ship.y + (-sin * 0.7 + cos * 0.7) * SHIP_RADIUS,
    );
    let right = (
        ship.x + (-cos * 0.7 + sin * 0.7) * SHIP_RADIUS,
        ship.y + (-sin * 0.7 - cos * 0.7) * SHIP_RADIUS,
    );

    let t = 2.0;
    draw_line(nose.0, nose.1, left.0, left.1, t, color);
    draw_line(left.0, left.1, right.0, right.1, t, color);
    draw_line(right.0, right.1, nose.0, nose.1, t, color);

    // Draw thrust flame when moving fast enough
    let speed = (ship.vx * ship.vx + ship.vy * ship.vy).sqrt();
    if speed > 30.0 {
        let tail = (
            ship.x - cos * SHIP_RADIUS * 1.3,
            ship.y - sin * SHIP_RADIUS * 1.3,
        );
        let flame_color = Color::new(1.0, 0.6, 0.1, 0.7);
        draw_line(left.0, left.1, tail.0, tail.1, 1.5, flame_color);
        draw_line(right.0, right.1, tail.0, tail.1, 1.5, flame_color);
    }
}

fn render_explosion(x: f32, y: f32, color: Color) {
    let faded = Color::new(color.r, color.g, color.b, 0.5);
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::PI / 3.0;
        let len = 8.0 + (i as f32 * 3.0) % 7.0;
        draw_line(
            x,
            y,
            x + angle.cos() * len,
            y + angle.sin() * len,
            1.5,
            faded,
        );
    }
}

fn render_projectiles(projectiles: &[Projectile]) {
    for p in projectiles {
        let color = if p.owner == 0 {
            Color::new(0.0, 1.0, 0.4, 0.9)
        } else {
            Color::new(0.4, 0.6, 1.0, 0.9)
        };
        draw_circle(p.x, p.y, PROJECTILE_RADIUS, color);
        // Small tail
        let speed = (p.vx * p.vx + p.vy * p.vy).sqrt().max(1.0);
        let dx = -p.vx / speed * 4.0;
        let dy = -p.vy / speed * 4.0;
        draw_line(
            p.x,
            p.y,
            p.x + dx,
            p.y + dy,
            1.0,
            Color::new(color.r, color.g, color.b, 0.4),
        );
    }
}

fn render_hud(state: &GameState, generation: usize, best_fitness: f32) {
    let text_color = Color::new(0.5, 0.5, 0.5, 1.0);
    draw_text(
        &format!("Gen: {}  Best: {:.0}", generation, best_fitness),
        10.0,
        20.0,
        20.0,
        text_color,
    );
    draw_text(
        &format!(
            "Time: {:.1}s / {:.0}s",
            state.time.min(MATCH_DURATION),
            MATCH_DURATION
        ),
        10.0,
        40.0,
        20.0,
        text_color,
    );

    let green = Color::new(0.0, 1.0, 0.4, 1.0);
    let blue = Color::new(0.4, 0.6, 1.0, 1.0);

    draw_text(
        &format!(
            "Green - Shots: {} Hits: {}",
            state.ships[0].shots_fired, state.ships[0].hits_scored
        ),
        10.0,
        ARENA_HEIGHT - 30.0,
        18.0,
        green,
    );
    draw_text(
        &format!(
            "Blue  - Shots: {} Hits: {}",
            state.ships[1].shots_fired, state.ships[1].hits_scored
        ),
        10.0,
        ARENA_HEIGHT - 10.0,
        18.0,
        blue,
    );
}

fn render_match_result(state: &GameState) {
    let msg = match state.winner {
        Some(0) => "GREEN WINS!",
        Some(1) => "BLUE WINS!",
        _ => "DRAW!",
    };

    let color = match state.winner {
        Some(0) => Color::new(0.0, 1.0, 0.4, 1.0),
        Some(1) => Color::new(0.4, 0.6, 1.0, 1.0),
        _ => Color::new(1.0, 1.0, 1.0, 1.0),
    };

    let font_size = 40.0;
    let text_width = measure_text(msg, None, font_size as u16, 1.0).width;
    draw_text(
        msg,
        (ARENA_WIDTH - text_width) / 2.0,
        ARENA_HEIGHT / 2.0,
        font_size,
        color,
    );
}
