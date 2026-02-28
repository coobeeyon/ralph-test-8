use rand::Rng;

use crate::game::*;
use crate::genome::*;

const SIM_DT: f32 = 1.0 / 60.0;
const SIM_STEPS: usize = (MATCH_DURATION / SIM_DT) as usize;

#[derive(Clone, Debug)]
pub struct MatchResult {
    pub fitness: [f32; 2],
}

/// Run a full match between two genomes at max speed, returning fitness for each
pub fn run_match(g1: &Genome, g2: &Genome, rng: &mut impl Rng) -> MatchResult {
    let mut state = GameState::new_random(rng);
    let genomes = [g1, g2];

    // Track proximity over time for engagement scoring
    let mut proximity_sum = [0.0f32; 2];
    let mut step_count = 0u32;

    for _ in 0..SIM_STEPS {
        if state.match_over {
            break;
        }

        let inputs0 = Genome::get_inputs(&state, 0);
        let inputs1 = Genome::get_inputs(&state, 1);
        let actions0 = genomes[0].evaluate(&inputs0);
        let actions1 = genomes[1].evaluate(&inputs1);
        state.update(SIM_DT, &[actions0, actions1]);

        // Accumulate proximity each step
        let dx = toroidal_diff(state.ships[0].x, state.ships[1].x, ARENA_WIDTH);
        let dy = toroidal_diff(state.ships[0].y, state.ships[1].y, ARENA_HEIGHT);
        let dist = (dx * dx + dy * dy).sqrt();
        let prox = 1.0 - (dist / 500.0).min(1.0);
        proximity_sum[0] += prox;
        proximity_sum[1] += prox;
        step_count += 1;
    }

    let avg_proximity = if step_count > 0 {
        [
            proximity_sum[0] / step_count as f32,
            proximity_sum[1] / step_count as f32,
        ]
    } else {
        [0.0, 0.0]
    };

    // Compute fitness for each ship
    let mut fitness = [0.0f32; 2];
    for i in 0..2 {
        let ship = &state.ships[i];
        let opp = &state.ships[1 - i];

        // Win bonus
        if ship.alive && !opp.alive {
            fitness[i] += 100.0;
        }

        // Death penalty
        if !ship.alive {
            fitness[i] -= 20.0;
        }

        // Hit bonus
        fitness[i] += ship.hits_scored as f32 * 50.0;

        // Accuracy bonus (reward aimed shots over spray)
        if ship.shots_fired > 0 {
            let accuracy = ship.hits_scored as f32 / ship.shots_fired as f32;
            fitness[i] += accuracy * 30.0;
        }

        // Active engagement: small reward for actually firing (prevents pure passive play)
        fitness[i] += (ship.shots_fired as f32).min(20.0) * 0.5;

        // Average proximity throughout the match (rewards aggressive positioning)
        fitness[i] += avg_proximity[i] * 20.0;

        // Survival time bonus (proportional, not binary)
        if ship.alive {
            fitness[i] += (state.time / MATCH_DURATION).min(1.0) * 15.0;
        } else {
            // Partial credit for surviving longer before dying
            fitness[i] += (state.time / MATCH_DURATION).min(1.0) * 5.0;
        }
    }

    MatchResult { fitness }
}
