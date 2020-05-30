use super::kdtree;
use super::tour::Tour;

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

        let search_result = frontier.nearest();
        if search_result.is_empty() {
            //println!("No nearest for city: #{:?}", id1);
            continue;
        }

        let nearest_city = search_result.first().unwrap();
        //println!("nearest to city.{:?} is {:?}", id1, nearest_city.id);
        //println!("alternatives: {:?}", frontier);

        let next_distance = city1.distance(&nearest_city);
        if next_distance < current_distance {
            if let Some(nearest_pos) = route.iter().position(|&x| x == nearest_city.id) {
                /*
                println!(
                    "swapping city.{:?} at {:?} <-> city.{:?} at {:?}",
                    id2,
                    i + 1,
                    nearest_city.id,
                    nearest_pos
                );
                */

                route.swap(i + 1, nearest_pos);
            }
        }
    }

    let tour = Tour::new(&route, cities);

    tour
}
