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

    for _ in 0..SIM_STEPS {
        if state.match_over {
            break;
        }

        let inputs0 = Genome::get_inputs(&state, 0);
        let inputs1 = Genome::get_inputs(&state, 1);
        let actions0 = genomes[0].evaluate(&inputs0);
        let actions1 = genomes[1].evaluate(&inputs1);
        state.update(SIM_DT, &[actions0, actions1]);
    }

    // Compute fitness for each ship
    let mut fitness = [0.0f32; 2];
    for i in 0..2 {
        let ship = &state.ships[i];
        let opp = &state.ships[1 - i];

        // Win bonus
        if ship.alive && !opp.alive {
            fitness[i] += 100.0;
        }

        // Hit bonus
        fitness[i] += ship.hits_scored as f32 * 50.0;

        // Survival bonus (survived to timeout)
        if ship.alive && state.time >= MATCH_DURATION {
            fitness[i] += 20.0;
        }

        // Accuracy bonus
        if ship.shots_fired > 0 {
            let accuracy = ship.hits_scored as f32 / ship.shots_fired as f32;
            fitness[i] += accuracy * 30.0;
        }

        // Engagement bonus: reward being closer to opponent on average
        // (approximated by final distance - closer = more engaged)
        let dx = toroidal_diff(ship.x, opp.x, ARENA_WIDTH);
        let dy = toroidal_diff(ship.y, opp.y, ARENA_HEIGHT);
        let dist = (dx * dx + dy * dy).sqrt();
        let proximity = 1.0 - (dist / 500.0).min(1.0);
        fitness[i] += proximity * 10.0;
    }

    MatchResult { fitness }
}
