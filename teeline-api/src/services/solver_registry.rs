use teeline::tsp::list_solvers;

use super::SolverRegistryService;
use crate::models::response::AlgorithmInfo;

pub struct SolverRegistry;

impl SolverRegistryService for SolverRegistry {
    fn list(&self) -> Vec<AlgorithmInfo> {
        list_solvers()
            .iter()
            .map(|si| AlgorithmInfo {
                name: si.name.to_string(),
                alias: si.alias.to_string(),
                category: si.category.to_string(),
                desc: si.desc.to_string(),
                complexity: si.complexity.to_string(),
                has_options: si.has_options,
                exact: si.exact,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_returns_non_empty() {
        let registry = SolverRegistry;
        let solvers = registry.list();
        assert!(!solvers.is_empty());
    }

    #[test]
    fn list_contains_nn() {
        let registry = SolverRegistry;
        let solvers = registry.list();
        let nn = solvers.iter().find(|s| s.alias == "nn");
        assert!(nn.is_some());
        assert_eq!(nn.unwrap().name, "Nearest Neighbor");
    }
}
