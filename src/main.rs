use std::fmt::Debug;
use std::str::FromStr;

use teeline::tsp::kdtree;

fn main() {
    let n_points = read_value::<usize>();
    let cities = read_cities(n_points);

    let tour = solve(&cities);
    print_solution(&tour, false);
}

fn print_solution(tour: &Tour, is_optimized: bool) {
    let optimization_flag = if is_optimized { 1 } else { 0 };

    println!("{} {}", tour.total, optimization_flag);
    for city_id in tour.route.iter() {
        print!("{} ", city_id);
    }

    print!("\n");
}

// k-opt

pub fn solve(cities: &[kdtree::KDPoint]) -> Tour {
    let search_tree = kdtree::build_tree(&cities);
    let n_nearest = 3;

    let mut route: Vec<usize> = cities.iter().map(|c| c.id).collect();

    // run optimization round
    for i in 0..(route.len() - 1) {
        let id1 = route[i];
        let city1 = cities[id1].clone();
        let frontier = search_tree.nearest(&city1, n_nearest);

        let id2 = route[i + 1];
        let city2 = cities[id2].clone();

        let current_distance = city1.distance(&city2);

        let nearest_city = frontier.nearest().take(1).next().unwrap();
        let next_distance = city1.distance(&nearest_city);
        if next_distance < current_distance {
            //println!("swaping {:?} <-> {:?}", id2, nearest_city.id);
            route.swap(id2, nearest_city.id);
        }
    }

    let tour = Tour::new(total_tour(&cities, &route), &route);

    tour
}

fn total_tour(cities: &[kdtree::KDPoint], route: &[usize]) -> f32 {
    let mut total = 0.0;
    let n_cities = route.len();

    for i in 1..n_cities {
        total += cities[i].distance(&cities[i - 1]);
    }

    total += cities[n_cities - 1].distance(&cities[0]);
    total
}

pub struct Tour {
    total: f32,
    route: Vec<usize>,
}

impl Tour {
    pub fn new(total: f32, route: &[usize]) -> Self {
        Tour {
            total,
            route: route.to_vec(),
        }
    }

    pub fn len(&self) -> usize {
        self.route.len()
    }
}

fn read_cities(n: usize) -> Vec<kdtree::KDPoint> {
    let mut rows = kdtree::PointMatrix::with_capacity(n);

    for _ in 0..n {
        rows.push(read_vector::<f32>());
    }

    let cities = kdtree::build_points(&rows);

    cities
}

fn read_value<T>() -> T
where
    T: FromStr,
    T::Err: Debug,
{
    let line = read_string();

    let res: T = line
        .trim()
        .parse::<T>()
        .expect("Failed to parse valur from stdin");

    res
}

fn read_vector<T>() -> Vec<T>
where
    T: FromStr,
    T::Err: Debug,
{
    let line = read_string();

    let res: Vec<T> = line
        .trim()
        .split_whitespace()
        .map(|token| token.parse::<T>().expect("Failed to parse vector row"))
        .collect();

    res
}

fn read_string() -> String {
    let mut buf = String::new();

    std::io::stdin()
        .read_line(&mut buf)
        .expect("Failed to read string from stding");

    buf
}
