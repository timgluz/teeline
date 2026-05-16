use super::route::Route;

#[derive(Debug, Clone)]
pub enum ProgressMessage {
    CityChange(usize),
    PathUpdate(Route, f32),
    EpochUpdate(usize),
    Done,
    Restart,
    OptimalTour(Vec<usize>),
}
