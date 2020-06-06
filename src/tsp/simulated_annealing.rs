use rand::seq::SliceRandom;
use rand::Rng;

use super::kdtree::KDPoint;
use super::route::Route;
use super::tour::{self, Tour};

pub fn solve(cities: &[KDPoint]) -> Tour {
    let mut best_route = Route::from_cities(cities);

    Tour::new(best_route.route(), cities)
}
