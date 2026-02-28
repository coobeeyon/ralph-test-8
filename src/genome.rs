use rand::Rng;

use crate::game::*;

pub const INPUT_SIZE: usize = 10;
pub const HIDDEN_SIZE: usize = 16;
pub const OUTPUT_SIZE: usize = 4;
// Weights: (INPUT+1)*HIDDEN + (HIDDEN+1)*OUTPUT = 11*16 + 17*4 = 176+68 = 244
pub const GENOME_SIZE: usize = (INPUT_SIZE + 1) * HIDDEN_SIZE + (HIDDEN_SIZE + 1) * OUTPUT_SIZE;

#[derive(Clone, Debug)]
pub struct Genome {
    pub weights: Vec<f32>,
    pub fitness: f32,
}

impl Genome {
    pub fn random(rng: &mut impl Rng) -> Self {
        Genome {
            weights: (0..GENOME_SIZE).map(|_| rng.gen_range(-1.0..1.0)).collect(),
            fitness: 0.0,
        }
    }

    /// Evaluate the neural network given sensor inputs, returning [thrust, turn_left, turn_right, fire]
    pub fn evaluate(&self, inputs: &[f32; INPUT_SIZE]) -> [f32; OUTPUT_SIZE] {
        let mut idx = 0;

        // Hidden layer
        let mut hidden = [0.0f32; HIDDEN_SIZE];
        for h in hidden.iter_mut() {
            let mut sum = 0.0;
            for &inp in inputs.iter() {
                sum += inp * self.weights[idx];
                idx += 1;
            }
            sum += self.weights[idx]; // bias
            idx += 1;
            *h = sum.tanh();
        }

        // Output layer
        let mut output = [0.0f32; OUTPUT_SIZE];
        for o in output.iter_mut() {
            let mut sum = 0.0;
            for &h in hidden.iter() {
                sum += h * self.weights[idx];
                idx += 1;
            }
            sum += self.weights[idx]; // bias
            idx += 1;
            *o = sigmoid(sum);
        }

        output
    }

    /// Build sensor inputs for a ship from the current game state
    pub fn get_inputs(state: &GameState, ship_idx: usize) -> [f32; INPUT_SIZE] {
        let ship = &state.ships[ship_idx];
        let opp = &state.ships[1 - ship_idx];

        // Relative position using toroidal distance
        let dx = toroidal_diff(opp.x, ship.x, ARENA_WIDTH);
        let dy = toroidal_diff(opp.y, ship.y, ARENA_HEIGHT);
        let dist = (dx * dx + dy * dy).sqrt().max(1.0);

        // Angle from our ship to opponent, relative to our heading
        let angle_to_opp = dy.atan2(dx) - ship.rotation;

        // Opponent heading relative to vector from them to us
        let angle_opp_to_us = (-dy).atan2(-dx);
        let opp_facing_angle = opp.rotation - angle_opp_to_us;

        // Own speed and opponent speed
        let own_speed = (ship.vx * ship.vx + ship.vy * ship.vy).sqrt();
        let opp_speed = (opp.vx * opp.vx + opp.vy * opp.vy).sqrt();

        // Nearest enemy bullet
        let (bullet_dist, bullet_angle) = nearest_enemy_bullet(state, ship_idx);

        [
            (dist / 500.0).min(1.0),     // distance to opponent (normalized)
            angle_to_opp.sin(),           // angle to opponent (sin)
            angle_to_opp.cos(),           // angle to opponent (cos)
            opp_facing_angle.sin(),       // opponent facing direction (sin)
            opp_facing_angle.cos(),       // opponent facing direction (cos)
            (own_speed / 300.0).min(1.0), // own speed normalized
            (opp_speed / 300.0).min(1.0), // opponent speed normalized
            bullet_dist,                  // nearest bullet distance
            bullet_angle.sin(),           // nearest bullet angle (sin)
            bullet_angle.cos(),           // nearest bullet angle (cos)
        ]
    }

    pub fn crossover(a: &Genome, b: &Genome, rng: &mut impl Rng) -> Genome {
        let point = rng.gen_range(0..GENOME_SIZE);
        let mut weights = Vec::with_capacity(GENOME_SIZE);
        for i in 0..GENOME_SIZE {
            weights.push(if i < point { a.weights[i] } else { b.weights[i] });
        }
        Genome {
            weights,
            fitness: 0.0,
        }
    }

    pub fn mutate(&mut self, rate: f32, strength: f32, rng: &mut impl Rng) {
        for w in &mut self.weights {
            if rng.gen::<f32>() < rate {
                *w += rng.gen_range(-strength..strength);
                *w = w.clamp(-3.0, 3.0);
            }
        }
    }
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn nearest_enemy_bullet(state: &GameState, ship_idx: usize) -> (f32, f32) {
    let ship = &state.ships[ship_idx];
    let mut min_dist = f32::MAX;
    let mut best_angle = 0.0f32;

    for p in &state.projectiles {
        if p.owner == ship_idx {
            continue;
        }
        let dx = toroidal_diff(p.x, ship.x, ARENA_WIDTH);
        let dy = toroidal_diff(p.y, ship.y, ARENA_HEIGHT);
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < min_dist {
            min_dist = dist;
            best_angle = dy.atan2(dx) - ship.rotation;
        }
    }

    if min_dist == f32::MAX {
        (1.0, 0.0)
    } else {
        ((min_dist / 500.0).min(1.0), best_angle)
    }
}
