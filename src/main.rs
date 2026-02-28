use macroquad::prelude::*;

mod evolution;
mod game;
mod genome;
mod simulation;

use evolution::*;
use game::*;
use genome::*;

const END_DELAY: f32 = 2.0;

enum AppState {
    Evolving,
    Showcasing {
        match_state: GameState,
        genomes: [Genome; 2],
        end_timer: f32,
    },
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Evolved Spaceship Duel".to_string(),
        window_width: ARENA_WIDTH as i32,
        window_height: ARENA_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut rng = ::rand::thread_rng();
    let mut pop = Population::new(&mut rng);
    let mut app_state = AppState::Evolving;

    loop {
        match app_state {
            AppState::Evolving => {
                // Run one generation
                pop.evaluate(&mut rng);
                let (g1, g2) = pop.get_top_two();
                pop.evolve(&mut rng);

                println!(
                    "Generation {} | Best fitness: {:.1}",
                    pop.generation, pop.best_fitness
                );

                // Start a showcase match
                let match_state = GameState::new_random(&mut rng);
                app_state = AppState::Showcasing {
                    match_state,
                    genomes: [g1, g2],
                    end_timer: END_DELAY,
                };
            }
            AppState::Showcasing {
                ref mut match_state,
                ref genomes,
                ref mut end_timer,
            } => {
                let dt = get_frame_time().min(1.0 / 30.0);

                if !match_state.match_over {
                    // Step simulation
                    let inputs0 = Genome::get_inputs(match_state, 0);
                    let inputs1 = Genome::get_inputs(match_state, 1);
                    let actions0 = genomes[0].evaluate(&inputs0);
                    let actions1 = genomes[1].evaluate(&inputs1);
                    match_state.update(dt, &[actions0, actions1]);
                } else {
                    // Count down end delay
                    *end_timer -= dt;
                    // Keep advancing time for display
                    match_state.time += dt;
                }

                // Render
                clear_background(BLACK);
                render_arena();
                render_projectiles(&match_state.projectiles);
                render_ship(&match_state.ships[0], Color::new(0.0, 1.0, 0.4, 1.0));
                render_ship(&match_state.ships[1], Color::new(0.4, 0.6, 1.0, 1.0));
                render_hud(match_state, pop.generation, pop.best_fitness);

                if match_state.match_over {
                    render_match_result(match_state);
                }

                // Transition back to evolving
                if match_state.match_over && *end_timer <= 0.0 {
                    app_state = AppState::Evolving;
                }
            }
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
