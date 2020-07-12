use rand::Rng;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;

use super::distance_matrix::DistanceMatrix;
use super::kdtree::KDPoint;
use super::route::{random_position_pair, Route};
use super::{Solution, SolverOptions};

type FitnessFn = Rc<dyn Fn(&[usize]) -> f32>;

pub fn solve(cities: &[KDPoint], options: &SolverOptions) -> Solution {
    let evaluator = build_evaluator(cities);

    let population_size = cities.len();
    let population = TspPopulation::from_cities(cities, population_size, &evaluator);
    let best_candidate = solve_ga(&population, evaluator, options);

    Solution::new(best_candidate.genotype(), cities)
}

fn solve_ga(
    population: &TspPopulation,
    fitness_fn: FitnessFn,
    options: &SolverOptions,
) -> TspGenotype {
    let population_size = population.len();
    let mutation_prob = options.mutation_probability;
    let elite_size = options.n_elite;

    let mut epoch = 0;
    let mut current_population = population.clone();

    while epoch < options.epochs {
        let mut new_population = TspPopulation::with_capacity(population_size);

        // pass n-fittest directly into new population;
        current_population.sort();
        for elite in current_population.individuals().iter().take(elite_size) {
            new_population.add(elite.clone());
        }

        for _ in elite_size..(population_size / 2) {
            let parent1 = current_population.random_selection();
            let parent2 = current_population.random_selection();

            let (mut child1, mut child2) = ordered_crossover(&parent1, &parent2, &fitness_fn);
            if probability(mutation_prob) {
                child1.mutate()
            };
            if probability(mutation_prob) {
                child2.mutate()
            };

            new_population.add(child1);
            new_population.add(child2);
        }

        current_population = new_population;

        if options.verbose {
            println!(
                "GA: epoch.{:?} - best individual:\n{:?}",
                epoch,
                current_population.best()
            );
        }

        epoch += 1;
    }

    let best_candidate = current_population.best().clone();

    best_candidate
}

fn build_evaluator(cities: &[KDPoint]) -> Rc<dyn Fn(&[usize]) -> f32> {
    let dm = Rc::new(DistanceMatrix::from_cities(cities).unwrap());

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

    let range_a: HashSet<usize> = parent1[from..=to].iter().map(|x| x.clone()).collect();
    let range_b: HashSet<usize> = parent2[from..=to].iter().map(|x| x.clone()).collect();

    // copy cross-overs from parents;
    for i in from..=to {
        gene1[i] = parent2[i].clone();
        gene2[i] = parent1[i].clone();
    }

    // copy other values like rolling-shift to -> from
    let mut k = (to + 1) % gene_len;
    let mut j1 = k;
    let mut j2 = k;

    for i in 0..gene_len {
        // copy other values from parent1 into child1
        let x_a = parent1[k].clone();
        if !range_b.contains(&x_a) {
            gene1[j1] = x_a;
            j1 = (j1 + 1) % gene_len;
        }

        // copy other values from parent2 into child2
        let x_b = parent2[k].clone();
        if !range_a.contains(&x_b) {
            gene2[j2] = x_b;
            j2 = (j2 + 1) % gene_len;
        }

        k = (k + 1) % gene_len;
    }

    (gene1, gene2)
}

// returns true with given probability
fn probability(p: f32) -> bool {
    let mut rng = rand::thread_rng();

    p > rng.gen()
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

    pub fn from_cities(cities: &[KDPoint], n: usize, fitness_fn: &FitnessFn) -> TspPopulation {
        let mut population = TspPopulation::with_capacity(n);
        let initial_route = Route::from_cities(cities);

        for _ in 0..n {
            let random_route = initial_route.random_successor();
            let fitness = fitness_fn(random_route.route());
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
        self.individuals.iter().map(|x| x.fitness.clone()).sum()
    }

    fn sort(&mut self) {
        self.individuals
            .sort_by(|x, y| y.fitness.partial_cmp(&x.fitness).unwrap_or(Ordering::Equal));
    }

    // TODO: test that entropy is good enough
    // roulette wheel selection
    fn random_selection(&self) -> &TspGenotype {
        let total = self.total_fitness();
        let mut rng = rand::thread_rng();

        let r = rng.gen_range(0.0, total);
        let mut up_to = 0.0;

        let mut candidate = self.individuals.last().unwrap();
        for g in self.individuals.iter() {
            if r < up_to + g.fitness() {
                candidate = g;
                break;
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

    pub fn set_fitness(&mut self, new_fitness: f32) {
        self.fitness = new_fitness;
    }

    // RSM from the reference paper
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
