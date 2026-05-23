use rand::Rng;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::mpsc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::probability::probability;
use super::progress::ProgressMessage;
use super::route::{random_position_pair, Route};
use super::{GAOptions, Solution, TspProblem};

type FitnessFn = Rc<dyn Fn(&[usize]) -> f32>;

pub fn solve(
    problem: &TspProblem,
    opts: &GAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> Solution {
    let cities = &problem.cities;
    let distances = &problem.distances;
    let evaluator = build_evaluator(distances);

    let population_size = cities.len();
    let population = match problem.initial_tour.as_deref() {
        Some(t) => TspPopulation::from_cities_seeded(cities, population_size, &evaluator, t),
        None => TspPopulation::from_cities(cities, population_size, &evaluator),
    };
    let best_candidate = solve_ga(&population, evaluator, distances, opts, progress_tx);

    let best_route = Route::new(best_candidate.genotype());
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(
            best_route,
            distances.tour_length(best_candidate.genotype()),
        ));
        let _ = tx.send(ProgressMessage::Done);
    }
    Solution::from_parts(best_candidate.genotype(), cities, distances)
}

fn solve_ga(
    population: &TspPopulation,
    fitness_fn: FitnessFn,
    distances: &DistanceMatrix,
    ga: &GAOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
) -> TspGenotype {
    let population_size = population.len();
    let mutation_prob = ga.mutation_probability;
    let elite_size = ga.n_elite;

    tracing::info!(
        population_size,
        elite_size = ga.n_elite,
        mutation_prob = ga.mutation_probability,
        "GA starting"
    );

    let mut epoch = 0;
    let mut current_population = population.clone();

    while epoch < ga.heuristic.epochs {
        let mut new_population = TspPopulation::with_capacity(population_size);

        current_population.sort();
        for elite in current_population.individuals().iter().take(elite_size) {
            new_population.add(elite.clone());
        }

        for _ in elite_size..(population_size / 2) {
            let parent1 = current_population.random_selection();
            let parent2 = current_population.random_selection();

            let (mut child1, mut child2) = ordered_crossover(parent1, parent2, &fitness_fn);
            if probability(mutation_prob) { child1.mutate() };
            if probability(mutation_prob) { child2.mutate() };

            new_population.add(child1);
            new_population.add(child2);
        }

        current_population = new_population;

        let best_candidate = current_population.best().clone();
        let best_route = Route::new(best_candidate.genotype());
        if let Some(tx) = progress_tx {
            let _ = tx.send(ProgressMessage::PathUpdate(
                best_route,
                distances.tour_length(best_candidate.genotype()),
            ));
        }

        tracing::debug!(epoch, fitness = current_population.best().fitness(), "GA: generation");

        epoch += 1;
    }

    current_population.best().clone()
}

fn build_evaluator(distances: &DistanceMatrix) -> FitnessFn {
    let dm = Rc::new(distances.clone());

    Rc::new(move |path: &[usize]| {
        let tour_length = dm.tour_length(path);

        if tour_length == 0.0 {
            0.0
        } else {
            1.0 / tour_length
        }
    })
}

fn ordered_crossover(
    parent1: &TspGenotype,
    parent2: &TspGenotype,
    fitness_fn: &FitnessFn,
) -> (TspGenotype, TspGenotype) {
    let (from, to) = random_position_pair(parent1.len());
    let (gene1, gene2) = ordered_crossover_genes(parent1.genotype(), parent2.genotype(), from, to);

    let child1 = TspGenotype::new(fitness_fn(&gene1[..]), &gene1);
    let child2 = TspGenotype::new(fitness_fn(&gene2[..]), &gene2);

    (child1, child2)
}

fn ordered_crossover_genes(
    parent1: &[usize],
    parent2: &[usize],
    from: usize,
    to: usize,
) -> (Vec<usize>, Vec<usize>) {
    let gene_len = parent1.len();
    let mut gene1: Vec<usize> = vec![0; gene_len];
    let mut gene2: Vec<usize> = vec![0; gene_len];

    let range_a: HashSet<usize> = parent1[from..=to].iter().copied().collect();
    let range_b: HashSet<usize> = parent2[from..=to].iter().copied().collect();

    gene1[from..=to].copy_from_slice(&parent2[from..=to]);
    gene2[from..=to].copy_from_slice(&parent1[from..=to]);

    let mut k = (to + 1) % gene_len;
    let mut j1 = k;
    let mut j2 = k;

    for _ in 0..gene_len {
        let x_a = parent1[k];
        if !range_b.contains(&x_a) {
            gene1[j1] = x_a;
            j1 = (j1 + 1) % gene_len;
        }

        let x_b = parent2[k];
        if !range_a.contains(&x_b) {
            gene2[j2] = x_b;
            j2 = (j2 + 1) % gene_len;
        }

        k = (k + 1) % gene_len;
    }

    (gene1, gene2)
}

#[derive(Debug, Clone)]
pub struct TspPopulation {
    n: usize,
    individuals: Vec<TspGenotype>,
}

impl TspPopulation {
    pub fn with_capacity(n: usize) -> Self {
        TspPopulation {
            n,
            individuals: Vec::with_capacity(n),
        }
    }

    pub fn from_cities(cities: &[KDPoint], n: usize, fitness_fn: &FitnessFn) -> TspPopulation {
        let mut population = TspPopulation::with_capacity(n);
        let base = Route::from_cities(cities);

        for _ in 0..n {
            let mut r = base.clone();
            r.shuffle();
            let fitness = fitness_fn(r.route());
            population.add(TspGenotype::new(fitness, r.route()));
        }

        population
    }

    pub fn from_cities_seeded(
        cities: &[KDPoint],
        n: usize,
        fitness_fn: &FitnessFn,
        seed: &[usize],
    ) -> TspPopulation {
        let mut population = TspPopulation::with_capacity(n);
        let n_seeded = (n / 5).max(1);

        population.add(TspGenotype::new(fitness_fn(seed), seed));

        for _ in 1..n_seeded {
            let mut mutant = TspGenotype::new(0.0, seed);
            let n_mutations = rand::rng().random_range(2..=4);
            for _ in 0..n_mutations {
                mutant.mutate();
            }
            let fitness = fitness_fn(mutant.genotype());
            mutant.set_fitness(fitness);
            population.add(mutant);
        }

        let base = Route::from_cities(cities);
        for _ in n_seeded..n {
            let mut r = base.clone();
            r.shuffle();
            population.add(TspGenotype::new(fitness_fn(r.route()), r.route()));
        }

        population
    }

    pub fn add(&mut self, individual: TspGenotype) {
        if self.n > self.individuals.len() {
            self.individuals.push(individual);
        }
    }

    pub fn len(&self) -> usize {
        self.individuals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.individuals.is_empty()
    }

    pub fn individuals(&self) -> &[TspGenotype] {
        &self.individuals
    }

    pub fn best(&self) -> &TspGenotype {
        self.individuals
            .iter()
            .max_by(|x, y| x.fitness.partial_cmp(&y.fitness).unwrap_or(Ordering::Equal))
            .unwrap()
    }

    fn total_fitness(&self) -> f32 {
        self.individuals.iter().map(|x| x.fitness).sum()
    }

    fn sort(&mut self) {
        self.individuals
            .sort_by(|x, y| y.fitness.partial_cmp(&x.fitness).unwrap_or(Ordering::Equal));
    }

    fn random_selection(&self) -> &TspGenotype {
        let total = self.total_fitness();
        let mut rng = rand::rng();

        let r = rng.random_range(0.0..total);
        let mut up_to = 0.0;

        let mut candidate = self.individuals.last().unwrap();
        for g in self.individuals.iter() {
            if r < up_to + g.fitness() {
                candidate = g;
                break;
            }

            up_to += g.fitness
        }
        candidate
    }
}

#[derive(Debug, Clone)]
pub struct TspGenotype {
    fitness: f32,
    genotype: Vec<usize>,
}

impl TspGenotype {
    pub fn new(fitness: f32, path: &[usize]) -> Self {
        TspGenotype {
            fitness,
            genotype: path.to_vec(),
        }
    }

    pub fn genotype(&self) -> &[usize] {
        &self.genotype[..]
    }

    pub fn fitness(&self) -> f32 {
        self.fitness
    }

    pub fn len(&self) -> usize {
        self.genotype.len()
    }

    pub fn is_empty(&self) -> bool {
        self.genotype.is_empty()
    }

    pub fn set_fitness(&mut self, new_fitness: f32) {
        self.fitness = new_fitness;
    }

    pub fn mutate(&mut self) {
        let (mut from, mut to) = random_position_pair(self.genotype.len());

        while from < to {
            self.genotype.swap(from, to);
            from += 1;
            to -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsp::{distance_matrix, kdtree, GAOptions, HeuristicOptions, TspProblem};

    fn tsp5_cities() -> Vec<KDPoint> {
        kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ])
    }

    #[test]
    fn test_ga_respects_initial_tour() {
        let cities = tsp5_cities();
        let dm = distance_matrix::from_cities(&cities);
        let seed: Vec<usize> = cities.iter().map(|c| c.id).collect();
        let dm_rc = std::rc::Rc::new(dm);
        let evaluator: FitnessFn = std::rc::Rc::new(move |path: &[usize]| {
            let tl = dm_rc.tour_length(path);
            if tl == 0.0 { 0.0 } else { 1.0 / tl }
        });
        let pop = TspPopulation::from_cities_seeded(&cities, cities.len(), &evaluator, &seed);
        assert_eq!(
            pop.individuals()[0].genotype(),
            seed.as_slice(),
            "first individual must be the exact seeded tour"
        );
    }

    #[test]
    fn test_solve_returns_real_tour_length_not_inverted_fitness() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);
        let distances = distance_matrix::from_cities(&cities);
        let opts = GAOptions {
            heuristic: HeuristicOptions { epochs: 100, ..HeuristicOptions::default() },
            ..GAOptions::default()
        };

        let problem = TspProblem::new(cities, distances.clone());
        let solution = solve(&problem, &opts, None);

        assert!(
            solution.total >= 3.9,
            "GA solution.total = {} looks like inverted fitness; expected >= 4.0",
            solution.total
        );
        let recomputed = distances.tour_length(solution.route());
        assert!(
            (solution.total - recomputed).abs() < 0.01,
            "solution.total ({}) != distances.tour_length ({}) — inconsistent",
            solution.total,
            recomputed
        );
    }

    fn make_evaluator(dm: distance_matrix::DistanceMatrix) -> FitnessFn {
        let dm_rc = std::rc::Rc::new(dm);
        std::rc::Rc::new(move |path: &[usize]| {
            let tl = dm_rc.tour_length(path);
            if tl == 0.0 { 0.0 } else { 1.0 / tl }
        })
    }

    #[test]
    fn test_seeded_population_n5_fraction() {
        let n = 20usize;
        let cities = kdtree::build_points(
            &(0..n).map(|i| vec![i as f32, 0.0]).collect::<Vec<_>>(),
        );
        let dm = distance_matrix::from_cities(&cities);
        let seed: Vec<usize> = (0..n).rev().collect();
        let evaluator = make_evaluator(dm);

        let trials = 100usize;
        let mut seed_derived_count = 0usize;
        for _ in 0..trials {
            let pop = TspPopulation::from_cities_seeded(&cities, n, &evaluator, &seed);
            for ind in pop.individuals() {
                if ind.genotype()[0] == n - 1 {
                    seed_derived_count += 1;
                }
            }
        }
        let avg_seed_derived = seed_derived_count as f32 / trials as f32;
        assert!(
            avg_seed_derived >= 2.5,
            "expected avg ≥ 2.5 seed-derived individuals per trial (n/5 fraction), got {:.2}",
            avg_seed_derived
        );
    }

    #[test]
    fn test_from_cities_produces_diverse_population() {
        let n = 20usize;
        let cities = kdtree::build_points(
            &(0..n).map(|i| vec![i as f32, 0.0]).collect::<Vec<_>>(),
        );
        let dm = distance_matrix::from_cities(&cities);
        let evaluator = make_evaluator(dm);

        let pop = TspPopulation::from_cities(&cities, n, &evaluator);

        let individuals = pop.individuals();
        let mut n_distinct_pairs = 0usize;
        let total_pairs = individuals.len() * (individuals.len() - 1) / 2;
        for i in 0..individuals.len() {
            for j in (i + 1)..individuals.len() {
                if individuals[i].genotype() != individuals[j].genotype() {
                    n_distinct_pairs += 1;
                }
            }
        }
        assert_eq!(
            n_distinct_pairs, total_pairs,
            "population has duplicate individuals; expected all {} pairs to be distinct, only {} were",
            total_pairs, n_distinct_pairs
        );
    }

    #[test]
    fn test_ordered_crossover_genes_with_example_from_book() {
        let parent1 = &[1, 2, 5, 3, 6, 4];
        let parent2 = &[5, 1, 4, 3, 6, 2];

        let (child1, child2) = ordered_crossover_genes(parent1, parent2, 2, 4);
        assert_eq!(vec![2, 5, 4, 3, 6, 1], child1);
        assert_eq!(vec![1, 4, 5, 3, 6, 2], child2);
    }

    #[test]
    fn test_ordered_crossover_genes_with_example_from_article() {
        let parent1 = &[9, 8, 4, 5, 6, 7, 1, 3, 2, 0];
        let parent2 = &[8, 7, 1, 2, 3, 0, 9, 5, 4, 6];

        let (child1, child2) = ordered_crossover_genes(parent1, parent2, 3, 5);
        assert_eq!(vec![5, 6, 7, 2, 3, 0, 1, 9, 8, 4], child1);
        assert_eq!(vec![2, 3, 0, 5, 6, 7, 9, 4, 8, 1], child2);
    }
}
