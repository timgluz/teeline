use crate::tsp::kdtree;

pub fn total_distance(cities: &[kdtree::KDPoint], route: &[usize]) -> f32 {
    let mut total = 0.0;
    let last_idx = route.len() - 1;

    for i in 0..last_idx {
        let distance = cities[route[i]].distance(&cities[route[i + 1]]);
        total += distance
    }

    total += cities[route[last_idx]].distance(&cities[route[0]]);

    total
}

pub struct Tour {
    pub total: f32,
    route: Vec<usize>,
    cities: Vec<kdtree::KDPoint>,
}

impl Tour {
    pub fn new(route: &[usize], cities: &[kdtree::KDPoint]) -> Self {
        let mut tour = Tour {
            total: 0.0,
            route: route.to_vec(),
            cities: cities.to_vec(),
        };

        tour.update_total();

        tour
    }

    pub fn len(&self) -> usize {
        self.route.len()
    }

    pub fn route(&self) -> &[usize] {
        self.route[..].as_ref()
    }

    pub fn cities(&self) -> &[kdtree::KDPoint] {
        &self.cities[..]
    }

    pub fn update_total(&mut self) {
        self.total = total_distance(self.cities(), self.route());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::assert_approx;

    #[test]
    fn total_distance_for_tsp_5_1() {
        let cities = kdtree::build_points(&[
            vec![0.0, 0.0],
            vec![0.0, 0.5],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            vec![1.0, 0.0],
        ]);

        let route = vec![0, 1, 2, 3, 4];

        assert_approx(4.0, total_distance(&cities, &route));
    }
}
