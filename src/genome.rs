use rand::Rng;

use crate::game::*;

pub const INPUT_SIZE: usize = 14;
pub const HIDDEN_SIZE: usize = 20;
pub const OUTPUT_SIZE: usize = 4;
// Weights: (INPUT+1)*HIDDEN + (HIDDEN+1)*OUTPUT = 15*20 + 21*4 = 300+84 = 384
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

        // Own speed and velocity direction relative to heading
        let own_speed = (ship.vx * ship.vx + ship.vy * ship.vy).sqrt();
        let own_vel_angle = if own_speed > 1.0 {
            ship.vy.atan2(ship.vx) - ship.rotation
        } else {
            0.0
        };

        let opp_speed = (opp.vx * opp.vx + opp.vy * opp.vy).sqrt();

        // Nearest enemy bullet
        let (bullet_dist, bullet_angle) = nearest_enemy_bullet(state, ship_idx);

        // Fire cooldown (0 = ready, 1 = max cooldown)
        let cooldown_norm = (ship.fire_cooldown / FIRE_COOLDOWN).min(1.0);

        // Own projectile count
        let own_projectiles = state.projectiles.iter().filter(|p| p.owner == ship_idx).count();
        let projectile_norm = own_projectiles as f32 / MAX_PROJECTILES_PER_SHIP as f32;

        [
            (dist / 500.0).min(1.0),      // 0: distance to opponent (normalized)
            angle_to_opp.sin(),            // 1: angle to opponent (sin)
            angle_to_opp.cos(),            // 2: angle to opponent (cos)
            opp_facing_angle.sin(),        // 3: opponent facing direction (sin)
            opp_facing_angle.cos(),        // 4: opponent facing direction (cos)
            (own_speed / 300.0).min(1.0),  // 5: own speed normalized
            (opp_speed / 300.0).min(1.0),  // 6: opponent speed normalized
            bullet_dist,                   // 7: nearest bullet distance
            bullet_angle.sin(),            // 8: nearest bullet angle (sin)
            bullet_angle.cos(),            // 9: nearest bullet angle (cos)
            own_vel_angle.sin(),           // 10: own drift direction (sin)
            own_vel_angle.cos(),           // 11: own drift direction (cos)
            cooldown_norm,                 // 12: fire cooldown (0=ready)
            projectile_norm,               // 13: own projectile count (normalized)
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
