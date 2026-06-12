use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    world: "solver",
    path: "wit",
});

struct HostState {
    table: wasmtime_wasi::ResourceTable,
    wasi: wasmtime_wasi::WasiCtx,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn make_engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).unwrap()
}

fn load_component(engine: &Engine) -> Component {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../target/wasm32-wasip2/debug/teeline_wasm.wasm"
    );
    Component::from_file(engine, path).expect(
        "WASM component not found — run: cd teeline-wasm && cargo component build",
    )
}

fn make_store(engine: &Engine) -> Store<HostState> {
    let wasi = WasiCtxBuilder::new().build();
    Store::new(
        engine,
        HostState {
            table: wasmtime_wasi::ResourceTable::new(),
            wasi,
        },
    )
}

fn five_cities() -> Vec<crate::teeline::solver::types::City> {
    use crate::teeline::solver::types::City;
    vec![
        City {
            id: 0,
            x: 565.0,
            y: 575.0,
        },
        City {
            id: 1,
            x: 25.0,
            y: 185.0,
        },
        City {
            id: 2,
            x: 345.0,
            y: 750.0,
        },
        City {
            id: 3,
            x: 945.0,
            y: 685.0,
        },
        City {
            id: 4,
            x: 845.0,
            y: 655.0,
        },
    ]
}

fn default_options() -> crate::teeline::solver::types::SolveOptions {
    crate::teeline::solver::types::SolveOptions {
        epochs: 200,
        platoo_epochs: 50,
        cooling_rate: 0.0001,
        max_temperature: 1000.0,
        min_temperature: 0.001,
        mutation_probability: 0.001,
        n_elite: 3,
        n_nearest: 3,
    }
}

fn assert_valid_tour(solution: &crate::teeline::solver::types::Solution, n_cities: usize) {
    assert_eq!(solution.route.len(), n_cities, "tour must visit all cities");
    assert!(solution.total > 0.0, "tour distance must be positive");
    let mut sorted: Vec<u32> = solution.route.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), n_cities, "each city must be visited exactly once");
}

fn run_solver(solver_name: &str) {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_solve(&mut store, solver_name, &five_cities(), default_options())
        .unwrap();
    let solution = result.unwrap_or_else(|e| panic!("{solver_name} returned error: {e}"));
    assert_valid_tour(&solution, 5);
}

#[test]
fn test_sa() {
    run_solver("sa")
}
#[test]
fn test_two_opt() {
    run_solver("two_opt")
}
#[test]
fn test_nearest_neighbor() {
    run_solver("nn")
}
#[test]
fn test_genetic_algorithm() {
    run_solver("ga")
}
#[test]
fn test_particle_swarm() {
    run_solver("pso")
}
#[test]
fn test_cuckoo_search() {
    run_solver("cs")
}
#[test]
fn test_flower_pollination() {
    run_solver("fpa")
}
#[test]
fn test_lin_kernighan() {
    run_solver("lk")
}
#[test]
fn test_tabu_search() {
    run_solver("tabu_search")
}
#[test]
fn test_stochastic_hill() {
    run_solver("stochastic_hill")
}
#[test]
fn test_bellman_karp() {
    run_solver("bhk")
}
#[test]
fn test_branch_bound() {
    run_solver("branch_bound")
}

#[test]
fn unknown_solver_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_solve(
            &mut store,
            "does_not_exist",
            &five_cities(),
            default_options(),
        )
        .unwrap();
    assert!(
        result.is_err(),
        "unknown solver must return Err, got {:?}",
        result.ok()
    );
}

// ── parse_and_solve integration tests ─────────────────────────────────────────

fn run_parse_and_solve(solver: &str, input: &str) -> crate::teeline::solver::types::Solution {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_parse_and_solve(&mut store, solver, input, default_options())
        .unwrap();
    result.unwrap_or_else(|e| panic!("parse_and_solve returned error: {e}"))
}

fn five_cities_json() -> String {
    r#"[{"id":0,"x":565.0,"y":575.0},{"id":1,"x":25.0,"y":185.0},{"id":2,"x":345.0,"y":750.0},{"id":3,"x":945.0,"y":685.0},{"id":4,"x":845.0,"y":655.0}]"#.to_string()
}

#[test]
fn test_parse_and_solve_tsplib_berlin52() {
    let input = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/berlin52.tsp"
    ))
    .expect("berlin52.tsp missing");
    let solution = run_parse_and_solve("nn", &input);
    assert_valid_tour(&solution, 52);
}

#[test]
fn test_parse_and_solve_json_5_cities() {
    let solution = run_parse_and_solve("nn", &five_cities_json());
    assert_valid_tour(&solution, 5);
}

#[test]
fn test_parse_and_solve_json_leading_whitespace() {
    let input = format!("  \n{}", five_cities_json());
    let solution = run_parse_and_solve("nn", &input);
    assert_valid_tour(&solution, 5);
}

#[test]
fn test_parse_and_solve_one_city_json_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_parse_and_solve(
            &mut store,
            "nn",
            r#"[{"id":0,"x":1.0,"y":2.0}]"#,
            default_options(),
        )
        .unwrap();
    assert!(result.is_err(), "single city must return Err");
}

#[test]
fn test_parse_and_solve_bad_solver_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_parse_and_solve(&mut store, "bogus", &five_cities_json(), default_options())
        .unwrap();
    assert!(result.is_err(), "unknown solver must return Err");
}

#[test]
fn test_parse_and_solve_empty_input_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_parse_and_solve(&mut store, "nn", "", default_options())
        .unwrap();
    assert!(result.is_err(), "empty input must return Err");
}

#[test]
fn test_parse_and_solve_invalid_json_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance
        .call_parse_and_solve(
            &mut store,
            "nn",
            r#"[{"id":0,"x":"bad","y":1.0}]"#,
            default_options(),
        )
        .unwrap();
    assert!(result.is_err(), "invalid JSON must return Err");
}

// ── list_algorithms and compare helpers ───────────────────────────────────────

const FIVE_CITIES_TSPLIB: &str = "NAME: test\n\
TYPE: TSP\n\
DIMENSION: 5\n\
EDGE_WEIGHT_TYPE: EUC_2D\n\
NODE_COORD_SECTION\n\
1 565.0 575.0\n\
2 25.0 185.0\n\
3 345.0 750.0\n\
4 945.0 685.0\n\
5 845.0 655.0\n\
EOF\n";

fn run_list_algorithms() -> Vec<crate::teeline::solver::types::AlgorithmInfo> {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    instance.call_list_algorithms(&mut store).unwrap()
}

fn run_compare(
    algorithms: &[&str],
    input: &str,
) -> Vec<crate::teeline::solver::types::CompareResult> {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let algos: Vec<String> = algorithms.iter().map(|&s| s.to_string()).collect();
    instance
        .call_compare(&mut store, &algos, input, default_options())
        .unwrap()
}

// ── list_algorithms tests ─────────────────────────────────────────────────────

#[test]
fn test_list_algorithms_returns_all_solvers() {
    let algorithms = run_list_algorithms();
    assert_eq!(algorithms.len(), 16, "expected 16 solvers");
    let ids: Vec<&str> = algorithms.iter().map(|a| a.id.as_str()).collect();
    for expected_id in &[
        "nn", "2opt", "3opt", "sa", "ga", "pso", "cs", "fpa",
        "tabu_search", "stochastic_hill", "shuffle", "bhk", "branch_bound", "lk", "or_opt",
        "christofides",
    ] {
        assert!(ids.contains(expected_id), "missing algorithm id: {}", expected_id);
    }
}

#[test]
fn test_list_algorithms_fields_non_empty() {
    let algorithms = run_list_algorithms();
    for algo in &algorithms {
        assert!(!algo.id.is_empty(),             "id empty for {:?}", algo.name);
        assert!(!algo.name.is_empty(),           "name empty for {}", algo.id);
        assert!(!algo.description.is_empty(),    "description empty for {}", algo.id);
        assert!(!algo.recommendation.is_empty(), "recommendation empty for {}", algo.id);
    }
}

// ── compare tests ─────────────────────────────────────────────────────────────

#[test]
fn test_compare_tsplib_input() {
    let results = run_compare(&["nn", "2opt"], FIVE_CITIES_TSPLIB);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].algorithm, "nn");
    assert_eq!(results[1].algorithm, "2opt");
    for r in &results {
        let sol = r.solution.as_ref().expect("solver should succeed");
        assert_valid_tour(sol, 5);
    }
}

#[test]
fn test_compare_preserves_algorithm_order() {
    let results = run_compare(&["sa", "nn", "ga"], FIVE_CITIES_TSPLIB);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].algorithm, "sa");
    assert_eq!(results[1].algorithm, "nn");
    assert_eq!(results[2].algorithm, "ga");
}

#[test]
fn test_compare_unknown_algorithm_returns_error_entry() {
    let results = run_compare(&["nn", "does_not_exist"], FIVE_CITIES_TSPLIB);
    assert_eq!(results.len(), 2);
    assert!(results[0].solution.is_ok(),  "nn should succeed");
    assert!(results[1].solution.is_err(), "unknown solver should return error entry");
}

#[test]
fn test_compare_invalid_input_all_error_entries() {
    let results = run_compare(&["nn", "2opt"], "not valid tsplib or json");
    assert_eq!(results.len(), 2);
    assert!(results[0].solution.is_err(), "should return parse error for nn");
    assert!(results[1].solution.is_err(), "should return parse error for 2opt");
}

#[test]
fn test_compare_json_input() {
    let results = run_compare(&["nn", "2opt"], &five_cities_json());
    assert_eq!(results.len(), 2);
    for r in &results {
        let sol = r.solution.as_ref().expect("solver should succeed with JSON input");
        assert_valid_tour(sol, 5);
    }
}

// ── parse integration tests ────────────────────────────────────────────────────

fn run_parse(input: &str) -> crate::teeline::solver::types::ParsedProblem {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    instance
        .call_parse(&mut store, input)
        .unwrap()
        .unwrap_or_else(|e| panic!("parse returned error: {e}"))
}

#[test]
fn test_parse_tsplib_berlin52() {
    let input = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/berlin52.tsp"
    ))
    .expect("berlin52.tsp missing");
    let p = run_parse(&input);
    assert_eq!(p.cities.len(), 52);
    assert_eq!(p.name, "berlin52");
    assert_eq!(p.distance_type, "EUC_2D");
}

#[test]
fn test_parse_tsplib_burma14() {
    let input = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/burma14.tsp"
    ))
    .expect("burma14.tsp missing");
    let p = run_parse(&input);
    assert_eq!(p.cities.len(), 14);
    assert_eq!(p.name, "burma14");
    assert_eq!(p.distance_type, "GEO");
}

#[test]
fn test_parse_json_5_cities() {
    let p = run_parse(&five_cities_json());
    assert_eq!(p.cities.len(), 5);
    assert_eq!(p.name, "");
    assert_eq!(p.distance_type, "");
}

#[test]
fn test_parse_empty_input_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    let result = instance.call_parse(&mut store, "").unwrap();
    assert!(result.is_err(), "empty input must return Err");
}

// ── get-version tests ─────────────────────────────────────────────────────────

fn run_get_version() -> String {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
    let mut store = make_store(&engine);
    let instance = Solver::instantiate(&mut store, &component, &linker).unwrap();
    instance.call_get_version(&mut store).unwrap()
}

#[test]
fn test_get_version_returns_non_empty() {
    let version = run_get_version();
    assert!(!version.is_empty(), "version string must not be empty");
}

// ── list_algorithms extended field tests ─────────────────────────────────────

#[test]
fn test_list_algorithms_kind_fields_present() {
    let algorithms = run_list_algorithms();
    let valid_kinds = ["exact", "constructive", "local-search", "metaheuristic", "utility"];
    for algo in &algorithms {
        assert!(
            valid_kinds.contains(&algo.kind.as_str()),
            "unexpected kind '{}' for solver '{}'",
            algo.kind, algo.id
        );
    }
}

#[test]
fn test_list_algorithms_sa_kind_and_params() {
    let algorithms = run_list_algorithms();
    let sa = algorithms.iter().find(|a| a.id == "sa").expect("sa missing");
    assert_eq!(sa.kind, "metaheuristic");
    let keys: Vec<&str> = sa.params.iter().map(|p| p.key.as_str()).collect();
    assert!(keys.contains(&"coolingRate"),    "sa must have coolingRate param");
    assert!(keys.contains(&"maxTemperature"), "sa must have maxTemperature param");
    assert!(keys.contains(&"minTemperature"), "sa must have minTemperature param");
    assert!(keys.contains(&"epochs"),         "sa must have epochs param");
    let cr = sa.params.iter().find(|p| p.key == "coolingRate").unwrap();
    assert_eq!(cr.value_type, "float", "coolingRate must be float type");
}

#[test]
fn test_list_algorithms_nn_kind_and_params() {
    let algorithms = run_list_algorithms();
    let nn = algorithms.iter().find(|a| a.id == "nn").expect("nn missing");
    assert_eq!(nn.kind, "constructive");
    assert!(nn.params.is_empty(), "nn must have no configurable params");
}

#[test]
fn test_list_algorithms_bhk_kind_and_params() {
    let algorithms = run_list_algorithms();
    let bhk = algorithms.iter().find(|a| a.id == "bhk").expect("bhk missing");
    assert_eq!(bhk.kind, "exact");
    assert!(bhk.params.is_empty(), "bhk must have no configurable params");
}

#[test]
fn test_list_algorithms_two_opt_kind() {
    let algorithms = run_list_algorithms();
    let two_opt = algorithms.iter().find(|a| a.id == "2opt").expect("2opt missing");
    assert_eq!(two_opt.kind, "local-search");
    assert!(!two_opt.params.is_empty(), "2opt must have heuristic params");
}

#[test]
fn test_list_algorithms_shuffle_kind() {
    let algorithms = run_list_algorithms();
    let shuffle = algorithms.iter().find(|a| a.id == "shuffle").expect("shuffle missing");
    assert_eq!(shuffle.kind, "utility");
}

const VALID_SOLVE_OPTIONS_KEYS: &[&str] = &[
    "epochs", "platooEpochs", "coolingRate", "maxTemperature",
    "minTemperature", "mutationProbability", "nElite", "nNearest",
];

#[test]
fn test_list_algorithms_all_param_keys_are_valid_solve_options_fields() {
    let algorithms = run_list_algorithms();
    for algo in &algorithms {
        for param in &algo.params {
            assert!(
                VALID_SOLVE_OPTIONS_KEYS.contains(&param.key.as_str()),
                "solver '{}' has unknown param key '{}' — must be a valid SolveOptions field",
                algo.id, param.key
            );
        }
    }
}

#[test]
fn test_list_algorithms_ga_params() {
    let algorithms = run_list_algorithms();
    let ga = algorithms.iter().find(|a| a.id == "ga").expect("ga missing");
    assert_eq!(ga.kind, "metaheuristic");
    let keys: Vec<&str> = ga.params.iter().map(|p| p.key.as_str()).collect();
    assert!(keys.contains(&"mutationProbability"), "ga must have mutationProbability");
    assert!(keys.contains(&"nElite"),             "ga must have nElite");
    let n_elite = ga.params.iter().find(|p| p.key == "nElite").unwrap();
    assert_eq!(n_elite.value_type, "int", "nElite must be int type");
}
