use super::route::Route;

/// Progress updates. Payloads that identify cities carry **city IDs**, never dense positions.
#[derive(Debug, Clone)]
pub enum ProgressMessage {
    /// A city was visited; the payload is a city ID.
    CityChange(usize),
    /// Current best route (city IDs in visit order) and its cost.
    PathUpdate(Route, f32),
    /// Iteration counter.
    EpochUpdate(usize),
    Done,
    Restart,
    /// Optimal comparison tour; city IDs.
    OptimalTour(Vec<usize>),
}
