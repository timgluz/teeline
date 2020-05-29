use std::fmt::Debug;
use std::str::FromStr;

use teeline::tsp;

fn main() {
    let n_points = read_value::<usize>();
    let mut points = tsp::kdtree::PointMatrix::with_capacity(n_points);

    for _ in 0..n_points {
        points.push(read_vector::<f32>());
    }

    let search_tree = tsp::kdtree::build_tree(points);
    let needle = tsp::kdtree::KDPoint::new(&[933_550.0, 977_200.0]);
    let res = search_tree.nearest(&needle, 3);
    println!("The nearest to (933_550, 977_200): #{:?}", res.point);

    let nearest: Vec<&tsp::kdtree::KDPoint> = res.nearest().collect();
    println!("Other points: #{:?}", nearest);
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
