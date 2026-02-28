use rand::Rng;

use crate::genome::*;
use crate::simulation::*;

const POPULATION_SIZE: usize = 100;
const MATCHES_PER_EVAL: usize = 8;
const TOURNAMENT_SIZE: usize = 5;
const ELITE_COUNT: usize = 5;
const MUTATION_RATE: f32 = 0.15;
const MUTATION_STRENGTH: f32 = 0.4;
const CROSSOVER_RATE: f32 = 0.7;

pub struct Population {
    pub genomes: Vec<Genome>,
    pub generation: usize,
    pub best_fitness: f32,
}

impl Population {
    pub fn new(rng: &mut impl Rng) -> Self {
        let genomes = (0..POPULATION_SIZE).map(|_| Genome::random(rng)).collect();
        Population {
            genomes,
            generation: 0,
            best_fitness: 0.0,
        }
    }

    /// Evaluate all genomes by running matches against random opponents
    pub fn evaluate(&mut self, rng: &mut impl Rng) {
        // Reset fitness
        for g in &mut self.genomes {
            g.fitness = 0.0;
        }

        // Each genome plays MATCHES_PER_EVAL matches against random opponents
        for i in 0..POPULATION_SIZE {
            for _ in 0..MATCHES_PER_EVAL {
                let mut j = rng.gen_range(0..POPULATION_SIZE - 1);
                if j >= i {
                    j += 1;
                }

                let result = run_match(&self.genomes[i], &self.genomes[j], rng);
                self.genomes[i].fitness += result.fitness[0];
                self.genomes[j].fitness += result.fitness[1];
            }
        }

        // Normalize by number of matches played
        // (each genome plays MATCHES_PER_EVAL as player 0, plus some as player 1)
        // We'll just use raw totals for ranking - more matches = more fitness opportunity
        // which is fine since everyone plays roughly the same number

        self.best_fitness = self.genomes.iter().map(|g| g.fitness).fold(0.0f32, f32::max);
    }

    /// Create next generation through selection, crossover, and mutation
    pub fn evolve(&mut self, rng: &mut impl Rng) {
        // Sort by fitness descending
        self.genomes.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

        let mut new_genomes = Vec::with_capacity(POPULATION_SIZE);

        // Keep elites
        for i in 0..ELITE_COUNT {
            let mut elite = self.genomes[i].clone();
            elite.fitness = 0.0;
            new_genomes.push(elite);
        }

        // Fill rest with offspring
        while new_genomes.len() < POPULATION_SIZE {
            let parent1 = tournament_select(&self.genomes, rng);
            let parent2 = tournament_select(&self.genomes, rng);

            let mut child = if rng.gen::<f32>() < CROSSOVER_RATE {
                Genome::crossover(parent1, parent2, rng)
            } else {
                parent1.clone()
            };
            child.fitness = 0.0;

            child.mutate(MUTATION_RATE, MUTATION_STRENGTH, rng);
            new_genomes.push(child);
        }

        self.genomes = new_genomes;
        self.generation += 1;
    }

    /// Get the two best genomes for showcase
    pub fn get_top_two(&self) -> (Genome, Genome) {
        let mut sorted: Vec<&Genome> = self.genomes.iter().collect();
        sorted.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
        (sorted[0].clone(), sorted[1].clone())
    }
}

fn tournament_select<'a>(genomes: &'a [Genome], rng: &mut impl Rng) -> &'a Genome {
    let mut best = &genomes[rng.gen_range(0..genomes.len())];
    for _ in 1..TOURNAMENT_SIZE {
        let candidate = &genomes[rng.gen_range(0..genomes.len())];
        if candidate.fitness > best.fitness {
            best = candidate;
        }
    }
    best
}
