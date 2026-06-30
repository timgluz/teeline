use async_trait::async_trait;

use crate::models::{
    request::{CompareRequest, ParseRequest, SolveRequest},
    response::{AlgorithmInfo, CompareResponse, ParseResponse, SolveResponse},
};

#[async_trait]
pub trait TspSolverService: Send + Sync {
    async fn parse(&self, req: &ParseRequest) -> Result<ParseResponse, String>;
    async fn solve(&self, req: &SolveRequest) -> Result<SolveResponse, String>;
    async fn compare(&self, req: &CompareRequest) -> Result<CompareResponse, String>;
}

pub trait SolverRegistryService: Send + Sync {
    fn list(&self) -> Vec<AlgorithmInfo>;
}

pub mod solver_registry;
pub mod tsp_service;
pub use solver_registry::SolverRegistry;
pub use tsp_service::TspService;
