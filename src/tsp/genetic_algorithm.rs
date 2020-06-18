use rand::Rng;
use std::cmp::Ordering;
use std::collections::HashSet;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::route::{random_position_pair, Route};
use super::tour::{self, Tour};

// TODO: should come from configs
const MAX_EPOCH: usize = 1000;
const MUTATION_PROBALITY: f32 = 0.01;
const ELITIST_SIZE: usize = 3; // how many best candidates pass directly into new population;

pub fn solve(cities: &[KDPoint]) -> Tour {
    let distances = DistanceMatrix::from_cities(&cities);
    let route = Route::from_cities(cities);
    let population_size = 10; // TODO: it should come from settings
    let population = TspPopulation::from_cities(cities, population_size);
    let best_candidate = solve_ga(&population);

    Tour::new(best_candidate.genotype(), cities)
}

fn solve_ga(population: &TspPopulation) -> TspGenotype {
    let population_size = population.len();
    let mut best_candidate = population.best().clone();

    let mut epoch = 0;
    let mut current_population = population.clone();
    while epoch < MAX_EPOCH {
        let mut new_population = TspPopulation::with_capacity(population_size);

        // TODO: pass n-fittest directly into new population;

        for i in 0..population_size {
            let parent1 = current_population.random_selection();
            let parent2 = current_population.random_selection();

            let (mut child1, mut child2) = crossover(&parent1, &parent2);
            if probability(MUTATION_PROBALITY) {
                child1.mutate()
            };
            if probability(MUTATION_PROBALITY) {
                child2.mutate()
            };

            new_population.add(child1);
            new_population.add(child2);
        }
    }

    best_candidate
}

fn crossover(parent1: &TspGenotype, parent2: &TspGenotype) -> (TspGenotype, TspGenotype) {
    let gene_len = parent1.len();
    let mut gene1 = Vec::with_capacity(gene_len);
    let mut gene2 = Vec::with_capacity(gene_len);

    let (from, to) = random_position_pair(gene_len);

    let mut k = if to < gene_len { to + 1 } else { to };
    let mut j1 = k;
    let mut j2 = k;

    let range_a: HashSet<usize> = parent1.genotype()[from..to]
        .iter()
        .map(|x| x.clone())
        .collect();
    let range_b: HashSet<usize> = parent2.genotype()[from..to]
        .iter()
        .map(|x| x.clone())
        .collect();

    // TODO: finish
    for i in 0..gene_len {
        let x_a = parent1.genotype()[k].clone();
        if !range_b.contains(&x_a) {
            gene1[j1] = x_a;
            j1 += 1;
        }

        let x_b = parent2.genotype()[k].clone();
        if !range_a.contains(&x_b) {
            gene2[j2] = x_b;
            j2 += 1;
        }

        k += 1;
    }

    let fitness1 = 0.0; // TODO: finish
    let child1 = TspGenotype::new(fitness1, &gene1);
    let fitness2 = 0.0;
    let child2 = TspGenotype::new(fitness2, &gene2);

    (child1, child2)
}

// returns true with given probability
fn probability(p: f32) -> bool {
    let mut rng = rand::thread_rng();

    p > rng.gen()
}

fn calculate_fitness(cities: &[KDPoint], path: &[usize]) -> f32 {
    tour::total_distance(cities, path)
}

// Add Population, Genotype

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

    pub fn from_cities(cities: &[KDPoint], n: usize) -> TspPopulation {
        let mut population = TspPopulation::with_capacity(n);
        let initial_route = Route::from_cities(cities);

        for i in 0..n {
            let random_route = initial_route.random_successor();
            let fitness = calculate_fitness(cities, random_route.route());
            population.add(TspGenotype::new(fitness, random_route.route()));
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

    pub fn best(&self) -> &TspGenotype {
        self.individuals
            .iter()
            .max_by(|x, y| x.fitness.partial_cmp(&y.fitness).unwrap_or(Ordering::Equal))
            .unwrap()
    }

    fn total_fitness(&self) -> f32 {
        self.individuals.iter().map(|x| x.fitness.clone()).sum()
    }

    // roulette wheel selection
    fn random_selection(&self) -> &TspGenotype {
        let total = self.total_fitness();
        let mut rng = rand::thread_rng();

        let r = rng.gen_range(0.0, total);
        let mut up_to = 0.0;

        let mut candidate;
        for g in self.individuals.iter() {
            // it always guaranteed to return item
            if r <= up_to + g.fitness() {
                candidate = g;
            }

            up_to += g.fitness
        }

        &candidate
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

    // RSM from the reference paper
    pub fn mutate(&mut self) {
        let (from, to) = random_position_pair(self.genotype.len());

        while from < to {
            self.genotype.swap(from, to);
            from += 1;
            to -= 1;
        }
    }
}
