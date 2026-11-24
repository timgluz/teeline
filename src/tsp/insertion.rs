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

pub fn solve(
    problem: &TspProblem,
    options: &InsertionOptions,
    progress_tx: Option<&mpsc::Sender<ProgressMessage>>,
    _init_tour: Option<&[usize]>,
) -> Solution {
    let cities = &problem.cities;
    let n = cities.len();
    let distance = |a: usize, b: usize| {
        f64::from(
            problem
                .distances
                .distance_by_pos(Position::new(a), Position::new(b))
                .expect("valid city positions"),
        )
    };
    let mut tour = Vec::with_capacity(n);
    if n <= 2 {
        tour.extend(0..n);
    } else {
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
                if (if options.strategy == InsertionStrategy::Farthest {
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
        let mut visited = vec![false; n];
        visited[seed.0] = true;
        visited[seed.1] = true;
        let mut closest: Vec<f64> = (0..n)
            .map(|city| distance(city, seed.0).min(distance(city, seed.1)))
            .collect();
        let candidate = |city: usize, from: usize, to: usize| Candidate {
            cost: distance(from, city) + distance(city, to) - distance(from, to),
            city,
            from,
            to,
            ids: (cities[city].id, cities[from].id, cities[to].id),
        };
        let mut next = vec![usize::MAX; n];
        next[seed.0] = seed.1;
        next[seed.1] = seed.0;
        let mut heap = BinaryHeap::new();
        if options.strategy == InsertionStrategy::Cheapest {
            for (city, &is_visited) in visited.iter().enumerate() {
                if !is_visited {
                    heap.push(candidate(city, seed.0, seed.1));
                    heap.push(candidate(city, seed.1, seed.0));
                }
            }
        }
        while tour.len() < n {
            let best = if options.strategy == InsertionStrategy::Cheapest {
                loop {
                    let c = heap.pop().expect("an insertion candidate exists");
                    if !visited[c.city] && next[c.from] == c.to {
                        break c;
                    }
                }
            } else {
                let city = (0..n)
                    .filter(|&c| !visited[c])
                    .min_by(|&a, &b| {
                        let order = closest[a].total_cmp(&closest[b]);
                        (if options.strategy == InsertionStrategy::Farthest {
                            order.reverse()
                        } else {
                            order
                        })
                        .then_with(|| cities[a].id.cmp(&cities[b].id))
                    })
                    .expect("an unvisited city exists");
                (0..tour.len())
                    .map(|i| candidate(city, tour[i], tour[(i + 1) % tour.len()]))
                    .max()
                    .expect("a tour edge exists")
            };
            let index = tour.iter().position(|&city| city == best.from).unwrap();
            tour.insert(index + 1, best.city);
            visited[best.city] = true;
            next[best.from] = best.city;
            next[best.city] = best.to;
            for city in 0..n {
                if !visited[city] {
                    if options.strategy == InsertionStrategy::Cheapest {
                        heap.push(candidate(city, best.from, best.city));
                        heap.push(candidate(city, best.city, best.to));
                    } else {
                        closest[city] = closest[city].min(distance(city, best.city));
                    }
                }
            }
        }
    }
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
