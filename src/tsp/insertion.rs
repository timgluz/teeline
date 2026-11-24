use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::mpsc;

use super::progress::ProgressMessage;
use super::route::Route;
use super::{Position, Solution, TspProblem};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InsertionStrategy {
    Nearest,
    #[default]
    Farthest,
    Cheapest,
}

impl InsertionStrategy {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "nearest" => Ok(Self::Nearest),
            "farthest" => Ok(Self::Farthest),
            "cheapest" => Ok(Self::Cheapest),
            _ => Err(format!(
                "invalid insertion strategy `{value}` — expected nearest, farthest, or cheapest"
            )),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InsertionOptions {
    pub strategy: InsertionStrategy,
}

impl InsertionOptions {
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn from_toml(table: &toml::Table) -> Result<Self, String> {
        let mut options = Self::default();
        for (key, value) in table {
            match key.as_str() {
                "strategy" => {
                    options.strategy = InsertionStrategy::parse(
                        value
                            .as_str()
                            .ok_or("config: `strategy` must be a string")?,
                    )?;
                }
                _ => return Err(format!("config: unknown field `{key}` in [insertion]")),
            }
        }
        options.validate()?;
        Ok(options)
    }

    pub fn from_cli(args: &clap::ArgMatches) -> Result<Self, String> {
        let mut options = Self::default();
        if let Some(value) = args.get_one::<String>("strategy") {
            options.strategy = InsertionStrategy::parse(value)?;
        }
        options.validate()?;
        Ok(options)
    }
}

#[derive(Clone, Debug)]
struct Candidate {
    cost: f64,
    city: usize,
    from: usize,
    to: usize,
    ids: (usize, usize, usize),
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.ids.cmp(&self.ids))
    }
}

struct InsertionState<'a> {
    problem: &'a TspProblem,
    strategy: InsertionStrategy,
    visited: Vec<bool>,
    closest: Vec<f64>,
    next: Vec<usize>,
    heap: BinaryHeap<Candidate>,
}

impl<'a> InsertionState<'a> {
    fn new(problem: &'a TspProblem, strategy: InsertionStrategy, seed: (usize, usize)) -> Self {
        let n = problem.cities.len();
        let mut state = Self {
            problem,
            strategy,
            visited: vec![false; n],
            closest: (0..n)
                .map(|city| {
                    Self::distance(problem, city, seed.0).min(Self::distance(problem, city, seed.1))
                })
                .collect(),
            next: vec![usize::MAX; n],
            heap: BinaryHeap::new(),
        };
        state.visited[seed.0] = true;
        state.visited[seed.1] = true;
        state.next[seed.0] = seed.1;
        state.next[seed.1] = seed.0;
        if strategy == InsertionStrategy::Cheapest {
            for city in 0..n {
                if !state.visited[city] {
                    state.heap.push(state.candidate(city, seed.0, seed.1));
                    state.heap.push(state.candidate(city, seed.1, seed.0));
                }
            }
        }
        state
    }

    fn distance(problem: &TspProblem, a: usize, b: usize) -> f64 {
        f64::from(
            problem
                .distances
                .distance_by_pos(Position::new(a), Position::new(b))
                .expect("valid city positions"),
        )
    }

    fn candidate(&self, city: usize, from: usize, to: usize) -> Candidate {
        let distance = |a, b| Self::distance(self.problem, a, b);
        let cities = &self.problem.cities;
        Candidate {
            cost: distance(from, city) + distance(city, to) - distance(from, to),
            city,
            from,
            to,
            ids: (cities[city].id, cities[from].id, cities[to].id),
        }
    }

    fn best_candidate(&mut self, tour: &[usize]) -> Candidate {
        match self.strategy {
            InsertionStrategy::Cheapest => self.cheapest_candidate(),
            InsertionStrategy::Nearest | InsertionStrategy::Farthest => {
                self.distance_candidate(tour)
            }
        }
    }

    fn cheapest_candidate(&mut self) -> Candidate {
        loop {
            let candidate = self.heap.pop().expect("an insertion candidate exists");
            if !self.visited[candidate.city] && self.next[candidate.from] == candidate.to {
                return candidate;
            }
        }
    }

    fn distance_candidate(&self, tour: &[usize]) -> Candidate {
        let city = (0..self.visited.len())
            .filter(|&city| !self.visited[city])
            .min_by(|&a, &b| {
                let order = self.closest[a].total_cmp(&self.closest[b]);
                (if self.strategy == InsertionStrategy::Farthest {
                    order.reverse()
                } else {
                    order
                })
                .then_with(|| self.problem.cities[a].id.cmp(&self.problem.cities[b].id))
            })
            .expect("an unvisited city exists");
        (0..tour.len())
            .map(|i| self.candidate(city, tour[i], tour[(i + 1) % tour.len()]))
            .max()
            .expect("a tour edge exists")
    }

    fn record_insertion(&mut self, best: &Candidate) {
        self.visited[best.city] = true;
        self.next[best.from] = best.city;
        self.next[best.city] = best.to;
        for city in 0..self.visited.len() {
            if !self.visited[city] {
                if self.strategy == InsertionStrategy::Cheapest {
                    self.heap.push(self.candidate(city, best.from, best.city));
                    self.heap.push(self.candidate(city, best.city, best.to));
                } else {
                    self.closest[city] =
                        self.closest[city].min(Self::distance(self.problem, city, best.city));
                }
            }
        }
    }
}

fn build_insertion_tour(problem: &TspProblem, strategy: InsertionStrategy) -> Vec<usize> {
    let cities = &problem.cities;
    let n = cities.len();
    if n <= 2 {
        return (0..n).collect();
    }

    let distance = |a, b| InsertionState::distance(problem, a, b);
    let mut tour = Vec::with_capacity(n);
    let mut seed = (0, 1);
    let mut seed_distance = distance(0, 1);
    let edge_ids = |a: usize, b: usize| {
        let (a, b) = (cities[a].id, cities[b].id);
        (a.min(b), a.max(b))
    };
    for a in 0..n {
        for b in a + 1..n {
            let d = distance(a, b);
            let order = d.total_cmp(&seed_distance);
            if (if strategy == InsertionStrategy::Farthest {
                order.is_gt()
            } else {
                order.is_lt()
            }) || (order.is_eq() && edge_ids(a, b) < edge_ids(seed.0, seed.1))
            {
                seed = (a, b);
                seed_distance = d;
            }
        }
    }
    if cities[seed.0].id > cities[seed.1].id {
        seed = (seed.1, seed.0);
    }
    tour.extend([seed.0, seed.1]);
    let mut state = InsertionState::new(problem, strategy, seed);
    while tour.len() < n {
        let best = state.best_candidate(&tour);
        let index = tour.iter().position(|&city| city == best.from).unwrap();
        tour.insert(index + 1, best.city);
        state.record_insertion(&best);
    }
    tour
}

pub fn solve(
    problem: &TspProblem,
    options: &InsertionOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let tour = build_insertion_tour(problem, options.strategy);
    let path: Vec<usize> = tour.iter().map(|&pos| cities[pos].id).collect();
    let solution = Solution::from_parts(&path, problem.distances.as_ref());
    if let Some(tx) = progress_tx {
        let _ = tx.send(ProgressMessage::PathUpdate(
            Route::new(&path),
            solution.total,
        ));
        let _ = tx.send(ProgressMessage::Done);
    }
    solution
}
