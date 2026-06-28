pub trait TspSolverService: Send + Sync {}
pub trait SolverRegistryService: Send + Sync {}

pub struct StubTspSolverService;
impl TspSolverService for StubTspSolverService {}

pub struct StubSolverRegistryService;
impl SolverRegistryService for StubSolverRegistryService {}
