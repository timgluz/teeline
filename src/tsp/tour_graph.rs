//! Fixed-degree undirected tour representation.
//!
//! A tour is stored as two neighbours per city, which makes edge lookup and replacement O(1)
//! and keeps degree two an invariant rather than a convention. Shared by the memetic EAX
//! crossover; available to other edge-based solvers that need to rewire a tour.

use super::DistanceSource;
use super::Position;

/// A fixed-degree undirected tour: `nbr[city]` holds exactly two neighbours.
#[derive(Clone, Debug)]
pub(crate) struct TourGraph {
    nbr: Vec<[usize; 2]>,
}

impl TourGraph {
    pub(crate) fn from_tour(tour: &[usize]) -> Self {
        let n = tour.len();
        let mut nbr = vec![[usize::MAX; 2]; n];
        for i in 0..n {
            let a = tour[i];
            let b = tour[(i + 1) % n];
            Self::push_neighbour(&mut nbr[a], b);
            Self::push_neighbour(&mut nbr[b], a);
        }
        debug_assert!(
            nbr.iter().all(|s| s[0] != usize::MAX && s[1] != usize::MAX),
            "every city must have two neighbours"
        );
        TourGraph { nbr }
    }

    fn push_neighbour(slots: &mut [usize; 2], value: usize) {
        if slots[0] == usize::MAX {
            slots[0] = value;
        } else {
            debug_assert_eq!(slots[1], usize::MAX, "degree must stay at two");
            slots[1] = value;
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.nbr.len()
    }

    pub(crate) fn neighbours(&self, city: usize) -> [usize; 2] {
        self.nbr[city]
    }

    pub(crate) fn has_edge(&self, a: usize, b: usize) -> bool {
        self.nbr[a][0] == b || self.nbr[a][1] == b
    }

    /// Replaces the neighbour `old` of `city` with `new`.
    pub(crate) fn replace_neighbour(&mut self, city: usize, old: usize, new: usize) {
        let slots = &mut self.nbr[city];
        if slots[0] == old {
            slots[0] = new;
        } else if slots[1] == old {
            slots[1] = new;
        } else {
            panic!("replace_neighbour: {old} is not a neighbour of {city}");
        }
    }

    pub(crate) fn to_tour(&self, start: usize) -> Vec<usize> {
        let n = self.nbr.len();
        let mut tour = Vec::with_capacity(n);
        let mut prev = usize::MAX;
        let mut current = start;
        for _ in 0..n {
            tour.push(current);
            let [a, b] = self.nbr[current];
            let next = if a != prev { a } else { b };
            prev = current;
            current = next;
        }
        debug_assert_eq!(current, start, "tour graph must be one cycle");
        tour
    }

    pub(crate) fn is_single_cycle(&self) -> bool {
        let n = self.nbr.len();
        if n == 0 {
            return true;
        }
        let mut seen = vec![false; n];
        let mut prev = usize::MAX;
        let mut current = 0;
        for _ in 0..n {
            if seen[current] {
                return false;
            }
            seen[current] = true;
            let [a, b] = self.nbr[current];
            let next = if a != prev { a } else { b };
            prev = current;
            current = next;
        }
        current == 0 && seen.into_iter().all(|s| s)
    }

    pub(crate) fn edge_sum(&self, distances: &dyn DistanceSource) -> f32 {
        let n = self.nbr.len();
        if n < 2 {
            return 0.0;
        }
        let tour = self.to_tour(0);
        (0..n)
            .map(|i| distance(distances, tour[i], tour[(i + 1) % n]))
            .sum()
    }

    /// Builds a graph from an explicit neighbour table. The caller is responsible for the
    /// degree-two and connectivity invariants; used by cross-module tests.
    #[cfg(test)]
    pub(crate) fn from_neighbours(nbr: Vec<[usize; 2]>) -> Self {
        TourGraph { nbr }
    }
}

pub(crate) fn distance(distances: &dyn DistanceSource, a: usize, b: usize) -> f32 {
    distances
        .distance_by_pos(Position::new(a), Position::new(b))
        .unwrap_or(f32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_round_trips_edges() {
        let tour = vec![3usize, 1, 4, 0, 2];
        let graph = TourGraph::from_tour(&tour);
        let rebuilt = graph.to_tour(3);
        for i in 0..tour.len() {
            assert!(graph.has_edge(tour[i], tour[(i + 1) % tour.len()]));
        }
        assert_eq!(rebuilt, tour);
    }

    #[test]
    fn graph_has_degree_two_for_every_city() {
        let tour = vec![0usize, 1, 2, 3, 4, 5];
        let graph = TourGraph::from_tour(&tour);
        for slots in &graph.nbr {
            assert_ne!(slots[0], usize::MAX);
            assert_ne!(slots[1], usize::MAX);
            assert_ne!(slots[0], slots[1]);
        }
    }

    #[test]
    fn replace_neighbour_swaps_the_edge() {
        let mut graph = TourGraph::from_tour(&[0usize, 1, 2, 3]);
        graph.replace_neighbour(0, 1, 2);
        assert!(graph.has_edge(0, 2));
        assert!(!graph.has_edge(0, 1));
    }

    #[test]
    fn is_single_cycle_rejects_disconnected_graph() {
        let graph =
            TourGraph::from_neighbours(vec![[1, 2], [0, 2], [0, 1], [4, 5], [3, 5], [3, 4]]);
        assert!(!graph.is_single_cycle());
    }

    #[test]
    #[should_panic(expected = "one cycle")]
    fn to_tour_panics_when_components_do_not_close() {
        // A triangle plus a square: walking the triangle does not close the 7-step tour.
        let graph = TourGraph::from_neighbours(vec![
            [1, 2],
            [0, 2],
            [0, 1],
            [4, 6],
            [3, 5],
            [4, 6],
            [3, 5],
        ]);
        let _ = graph.to_tour(0);
    }
}
