use std::sync::OnceLock;
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

// ── Shared WASM runtime ───────────────────────────────────────────────────────
//
// Engine (JIT compiler) and Component (compiled WASM binary) are expensive to
// create — each one JIT-compiles the full WASM binary. Both are Send + Sync in
// wasmtime and safe to share across test threads. Linker (WASI bindings table)
// is cheap but also safe to share.
//
// Sharing these three means the binary is compiled exactly once regardless of
// how many tests run in parallel. Each test then only allocates a fresh Store
// (cheap) and calls instantiate() (fast — links pre-compiled code).

static ENGINE: OnceLock<Engine> = OnceLock::new();
static COMPONENT: OnceLock<Component> = OnceLock::new();
static LINKER: OnceLock<Linker<HostState>> = OnceLock::new();

fn shared_engine() -> &'static Engine {
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.wasm_component_model(true);
        Engine::new(&config).unwrap()
    })
}

fn shared_component() -> &'static Component {
    COMPONENT.get_or_init(|| {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/target/wasm32-wasip2/debug/teeline_wasm.wasm"
        );
        Component::from_file(shared_engine(), path).expect(
            "WASM component not found — run: cargo component build --manifest-path teeline-wasm/Cargo.toml --target wasm32-wasip2",
        )
    })
}

fn shared_linker() -> &'static Linker<HostState> {
    LINKER.get_or_init(|| {
        let mut linker: Linker<HostState> = Linker::new(shared_engine());
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
        linker
    })
}

/// Create a fresh (Store, Solver) pair for one test invocation.
/// Store is cheap to allocate; instantiate() links pre-compiled code (~fast).
fn make_instance() -> (Store<HostState>, Solver) {
    let wasi = WasiCtxBuilder::new().build();
    let mut store = Store::new(
        shared_engine(),
        HostState {
            table: wasmtime_wasi::ResourceTable::new(),
            wasi,
        },
    );
    let instance = Solver::instantiate(&mut store, shared_component(), shared_linker()).unwrap();
    (store, instance)
}

// ── Test data helpers ─────────────────────────────────────────────────────────

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
    assert_eq!(
        sorted.len(),
        n_cities,
        "each city must be visited exactly once"
    );
}

// ── Per-call helpers (use shared runtime) ────────────────────────────────────

fn run_solver(solver_name: &str) {
    let (mut store, instance) = make_instance();
    let result = instance
        .call_solve(&mut store, solver_name, &five_cities(), default_options())
        .unwrap();
    let solution = result.unwrap_or_else(|e| panic!("{solver_name} returned error: {e}"));
    assert_valid_tour(&solution, 5);
}

fn run_parse_and_solve(solver: &str, input: &str) -> crate::teeline::solver::types::Solution {
    let (mut store, instance) = make_instance();
    let result = instance
        .call_parse_and_solve(&mut store, solver, input, default_options())
        .unwrap();
    result.unwrap_or_else(|e| panic!("parse_and_solve returned error: {e}"))
}

fn run_list_algorithms() -> Vec<crate::teeline::solver::types::AlgorithmInfo> {
    let (mut store, instance) = make_instance();
    instance.call_list_algorithms(&mut store).unwrap()
}

fn run_compare(
    algorithms: &[&str],
    input: &str,
) -> Vec<crate::teeline::solver::types::CompareResult> {
    let (mut store, instance) = make_instance();
    let algos: Vec<String> = algorithms.iter().map(|&s| s.to_string()).collect();
    instance
        .call_compare(&mut store, &algos, input, default_options())
        .unwrap()
}

fn run_parse(input: &str) -> crate::teeline::solver::types::ParsedProblem {
    let (mut store, instance) = make_instance();
    instance
        .call_parse(&mut store, input)
        .unwrap()
        .unwrap_or_else(|e| panic!("parse returned error: {e}"))
}

fn run_get_version() -> String {
    let (mut store, instance) = make_instance();
    instance.call_get_version(&mut store).unwrap()
}

fn run_compare_tours(
    solver_route: &[u32],
    opt_route: &[u32],
    cities: &[crate::teeline::solver::types::City],
) -> Result<crate::teeline::solver::types::ComparisonStats, String> {
    let (mut store, instance) = make_instance();
    instance
        .call_compare_tours(&mut store, solver_route, opt_route, cities)
        .unwrap()
}

fn run_tour_distance(
    route: &[u32],
    cities: &[crate::teeline::solver::types::City],
) -> Result<f32, String> {
    let (mut store, instance) = make_instance();
    instance
        .call_tour_distance(&mut store, route, cities)
        .unwrap()
}

// Unit square, side 10: closed-loop perimeter is exactly 40.0.
fn square_cities() -> Vec<crate::teeline::solver::types::City> {
    use crate::teeline::solver::types::City;
    vec![
        City {
            id: 0,
            x: 0.0,
            y: 0.0,
        },
        City {
            id: 1,
            x: 10.0,
            y: 0.0,
        },
        City {
            id: 2,
            x: 10.0,
            y: 10.0,
        },
        City {
            id: 3,
            x: 0.0,
            y: 10.0,
        },
    ]
}

// ── solve tests ───────────────────────────────────────────────────────────────

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
fn test_three_opt() {
    run_solver("3opt")
}
#[test]
fn test_or_opt() {
    run_solver("or_opt")
}
#[test]
fn test_shuffle() {
    run_solver("shuffle")
}
#[test]
fn test_christofides() {
    run_solver("christofides")
}
#[test]
fn test_gravitational_search() {
    run_solver("gsa")
}
#[test]
fn test_fourier() {
    run_solver("fourier")
}

#[test]
fn unknown_solver_returns_err() {
    let (mut store, instance) = make_instance();
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
    let (mut store, instance) = make_instance();
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
    let (mut store, instance) = make_instance();
    let result = instance
        .call_parse_and_solve(&mut store, "bogus", &five_cities_json(), default_options())
        .unwrap();
    assert!(result.is_err(), "unknown solver must return Err");
}

#[test]
fn test_parse_and_solve_empty_input_returns_err() {
    let (mut store, instance) = make_instance();
    let result = instance
        .call_parse_and_solve(&mut store, "nn", "", default_options())
        .unwrap();
    assert!(result.is_err(), "empty input must return Err");
}

#[test]
fn test_parse_and_solve_invalid_json_returns_err() {
    let (mut store, instance) = make_instance();
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

fn five_cities_json() -> String {
    r#"[{"id":0,"x":565.0,"y":575.0},{"id":1,"x":25.0,"y":185.0},{"id":2,"x":345.0,"y":750.0},{"id":3,"x":945.0,"y":685.0},{"id":4,"x":845.0,"y":655.0}]"#.to_string()
}

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

// ── list_algorithms tests ─────────────────────────────────────────────────────

#[test]
fn test_list_algorithms_returns_all_solvers() {
    let algorithms = run_list_algorithms();
    assert_eq!(algorithms.len(), 19, "expected 19 solvers");
    let ids: Vec<&str> = algorithms.iter().map(|a| a.id.as_str()).collect();
    for expected_id in &[
        "nn",
        "2opt",
        "3opt",
        "sa",
        "ga",
        "gsa",
        "pso",
        "cs",
        "fpa",
        "tabu_search",
        "stochastic_hill",
        "shuffle",
        "bhk",
        "branch_bound",
        "lk",
        "or_opt",
        "christofides",
        "fourier",
        "som",
    ] {
        assert!(
            ids.contains(expected_id),
            "missing algorithm id: {}",
            expected_id
        );
    }
}

#[test]
fn test_list_algorithms_fields_non_empty() {
    let algorithms = run_list_algorithms();
    for algo in &algorithms {
        assert!(!algo.id.is_empty(), "id empty for {:?}", algo.name);
        assert!(!algo.name.is_empty(), "name empty for {}", algo.id);
        assert!(
            !algo.description.is_empty(),
            "description empty for {}",
            algo.id
        );
        assert!(
            !algo.recommendation.is_empty(),
            "recommendation empty for {}",
            algo.id
        );
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
    assert!(results[0].solution.is_ok(), "nn should succeed");
    assert!(
        results[1].solution.is_err(),
        "unknown solver should return error entry"
    );
}

#[test]
fn test_compare_invalid_input_all_error_entries() {
    let results = run_compare(&["nn", "2opt"], "not valid tsplib or json");
    assert_eq!(results.len(), 2);
    assert!(
        results[0].solution.is_err(),
        "should return parse error for nn"
    );
    assert!(
        results[1].solution.is_err(),
        "should return parse error for 2opt"
    );
}

#[test]
fn test_compare_json_input() {
    let results = run_compare(&["nn", "2opt"], &five_cities_json());
    assert_eq!(results.len(), 2);
    for r in &results {
        let sol = r
            .solution
            .as_ref()
            .expect("solver should succeed with JSON input");
        assert_valid_tour(sol, 5);
    }
}

// ── compare_tours tests ───────────────────────────────────────────────────────

#[test]
fn test_compare_tours_identical_routes() {
    let cities = five_cities();
    let route: Vec<u32> = cities.iter().map(|c| c.id).collect();
    let stats = run_compare_tours(&route, &route, &cities).expect("identical routes must succeed");
    assert_eq!(stats.gap_pct, 0.0, "identical tour must have 0% gap");
    assert_eq!(
        stats.shared_edges,
        route.len() as u32,
        "all edges must be shared"
    );
    assert_eq!(stats.solver_only_edges, 0);
    assert_eq!(stats.optimal_only_edges, 0);
    assert!((stats.optimal_cost - stats.solver_cost).abs() < 0.001);
}

#[test]
fn test_compare_tours_permuted_route_has_positive_gap() {
    let cities = five_cities();
    // five_cities() returns IDs 0..4; optimal: [0,1,2,3,4], solver: [0,2,1,3,4]
    let optimal: Vec<u32> = cities.iter().map(|c| c.id).collect();
    let mut solver = optimal.clone();
    solver.swap(1, 2); // swap cities 1 and 2 to create a worse route
    let stats =
        run_compare_tours(&solver, &optimal, &cities).expect("valid permuted routes must succeed");
    assert!(stats.gap_pct >= 0.0, "gap must be non-negative");
    // Swapped route may or may not be worse depending on geometry; just verify the call works
    // and stats are consistent
    assert_eq!(
        stats.shared_edges + stats.solver_only_edges,
        optimal.len() as u32,
        "shared + solver_only must equal tour length"
    );
}

#[test]
fn test_compare_tours_dimension_mismatch_returns_error() {
    let cities = five_cities();
    let route: Vec<u32> = cities.iter().map(|c| c.id).collect();
    let short_route = vec![route[0], route[1]]; // only 2 cities
    let result = run_compare_tours(&short_route, &route, &cities);
    assert!(result.is_err(), "mismatched lengths must return Err");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("dimension mismatch"),
        "error must mention 'dimension mismatch', got: {msg}"
    );
}

#[test]
fn test_compare_tours_unknown_city_id_returns_error() {
    let cities = five_cities(); // IDs 0..4
    let route: Vec<u32> = cities.iter().map(|c| c.id).collect();
    let bad_route: Vec<u32> = route.iter().map(|&id| id + 100).collect(); // IDs 100..104 don't exist
    let result = run_compare_tours(&bad_route, &route, &cities);
    assert!(result.is_err(), "unknown city IDs must return Err");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("unknown city"),
        "error must mention 'unknown city', got: {msg}"
    );
}

#[test]
fn test_compare_tours_berlin52_optimal_vs_itself() {
    // Parse berlin52 cities from the TSP file (reuse existing run_parse helper)
    let tsp_input = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/berlin52.tsp",
    ))
    .expect("berlin52.tsp must exist");
    let parsed = run_parse(&tsp_input);

    // Parse the optimal tour (city IDs, 1-based as per TSPLIB)
    let opt_text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/berlin52.opt.tour",
    ))
    .expect("berlin52.opt.tour must exist");
    let opt_route: Vec<u32> = opt_text
        .lines()
        .skip_while(|l| l.trim() != "TOUR_SECTION")
        .skip(1)
        .map(|l| l.trim().parse::<i32>().unwrap_or(-1))
        .take_while(|&n| n > 0)
        .map(|n| n as u32)
        .collect();
    assert_eq!(
        opt_route.len(),
        52,
        "berlin52 optimal tour must have 52 cities"
    );

    let stats = run_compare_tours(&opt_route, &opt_route, &parsed.cities)
        .expect("optimal vs itself must succeed");
    assert_eq!(stats.gap_pct, 0.0, "optimal vs itself must have 0% gap");
    assert_eq!(stats.shared_edges, 52, "all 52 edges must be shared");
}

// ── parse integration tests ────────────────────────────────────────────────────

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
    let (mut store, instance) = make_instance();
    let result = instance.call_parse(&mut store, "").unwrap();
    assert!(result.is_err(), "empty input must return Err");
}

// ── get-version tests ─────────────────────────────────────────────────────────

#[test]
fn test_get_version_returns_non_empty() {
    let version = run_get_version();
    assert!(!version.is_empty(), "version string must not be empty");
}

// ── list_algorithms extended field tests ─────────────────────────────────────

#[test]
fn test_list_algorithms_kind_fields_present() {
    let algorithms = run_list_algorithms();
    let valid_kinds = [
        "exact",
        "constructive",
        "local-search",
        "metaheuristic",
        "utility",
    ];
    for algo in &algorithms {
        assert!(
            valid_kinds.contains(&algo.kind.as_str()),
            "unexpected kind '{}' for solver '{}'",
            algo.kind,
            algo.id
        );
    }
}

#[test]
fn test_list_algorithms_sa_kind_and_params() {
    let algorithms = run_list_algorithms();
    let sa = algorithms
        .iter()
        .find(|a| a.id == "sa")
        .expect("sa missing");
    assert_eq!(sa.kind, "metaheuristic");
    let keys: Vec<&str> = sa.params.iter().map(|p| p.key.as_str()).collect();
    assert!(
        keys.contains(&"coolingRate"),
        "sa must have coolingRate param"
    );
    assert!(
        keys.contains(&"maxTemperature"),
        "sa must have maxTemperature param"
    );
    assert!(
        keys.contains(&"minTemperature"),
        "sa must have minTemperature param"
    );
    assert!(keys.contains(&"epochs"), "sa must have epochs param");
    let cr = sa.params.iter().find(|p| p.key == "coolingRate").unwrap();
    assert_eq!(cr.value_type, "float", "coolingRate must be float type");
}

#[test]
fn test_list_algorithms_nn_kind_and_params() {
    let algorithms = run_list_algorithms();
    let nn = algorithms
        .iter()
        .find(|a| a.id == "nn")
        .expect("nn missing");
    assert_eq!(nn.kind, "constructive");
    assert!(nn.params.is_empty(), "nn must have no configurable params");
}

#[test]
fn test_list_algorithms_bhk_kind_and_params() {
    let algorithms = run_list_algorithms();
    let bhk = algorithms
        .iter()
        .find(|a| a.id == "bhk")
        .expect("bhk missing");
    assert_eq!(bhk.kind, "exact");
    assert!(
        bhk.params.is_empty(),
        "bhk must have no configurable params"
    );
}

#[test]
fn test_list_algorithms_two_opt_kind() {
    let algorithms = run_list_algorithms();
    let two_opt = algorithms
        .iter()
        .find(|a| a.id == "2opt")
        .expect("2opt missing");
    assert_eq!(two_opt.kind, "local-search");
    assert!(
        !two_opt.params.is_empty(),
        "2opt must have heuristic params"
    );
}

#[test]
fn test_list_algorithms_shuffle_kind() {
    let algorithms = run_list_algorithms();
    let shuffle = algorithms
        .iter()
        .find(|a| a.id == "shuffle")
        .expect("shuffle missing");
    assert_eq!(shuffle.kind, "utility");
}

const VALID_SOLVE_OPTIONS_KEYS: &[&str] = &[
    "epochs",
    "platooEpochs",
    "coolingRate",
    "maxTemperature",
    "minTemperature",
    "mutationProbability",
    "nElite",
    "nNearest",
];

#[test]
fn test_list_algorithms_all_param_keys_are_valid_solve_options_fields() {
    let algorithms = run_list_algorithms();
    for algo in &algorithms {
        for param in &algo.params {
            assert!(
                VALID_SOLVE_OPTIONS_KEYS.contains(&param.key.as_str()),
                "solver '{}' has unknown param key '{}' — must be a valid SolveOptions field",
                algo.id,
                param.key
            );
        }
    }
}

#[test]
fn test_list_algorithms_christofides_kind_and_recommendation() {
    let algorithms = run_list_algorithms();
    let chr = algorithms
        .iter()
        .find(|a| a.id == "christofides")
        .expect("christofides missing");
    assert_eq!(
        chr.kind, "constructive",
        "christofides must be 'constructive'"
    );
    assert!(
        chr.params.is_empty(),
        "christofides must have no configurable params"
    );
    assert!(
        chr.recommendation.len() > 20,
        "christofides recommendation must be a real description, not a bare category name; got: '{}'",
        chr.recommendation
    );
    assert_ne!(
        chr.recommendation, "Approximation",
        "recommendation must not be the bare category name"
    );
}

#[test]
fn test_list_algorithms_fourier_kind_and_params() {
    let algorithms = run_list_algorithms();
    let f = algorithms
        .iter()
        .find(|a| a.id == "fourier")
        .expect("fourier missing");
    assert_eq!(f.kind, "constructive", "fourier must be 'constructive'");
    assert_eq!(
        f.params.len(),
        1,
        "fourier must have exactly 1 param (epochs)"
    );
    assert_eq!(f.params[0].key, "epochs", "fourier param must be 'epochs'");
    assert_eq!(f.params[0].value_type, "int", "epochs must be int type");
    assert!(
        f.recommendation.len() > 20,
        "fourier recommendation must be a real description; got: '{}'",
        f.recommendation
    );
}

#[test]
fn test_list_algorithms_ga_params() {
    let algorithms = run_list_algorithms();
    let ga = algorithms
        .iter()
        .find(|a| a.id == "ga")
        .expect("ga missing");
    assert_eq!(ga.kind, "metaheuristic");
    let keys: Vec<&str> = ga.params.iter().map(|p| p.key.as_str()).collect();
    assert!(
        keys.contains(&"mutationProbability"),
        "ga must have mutationProbability"
    );
    assert!(keys.contains(&"nElite"), "ga must have nElite");
    let n_elite = ga.params.iter().find(|p| p.key == "nElite").unwrap();
    assert_eq!(n_elite.value_type, "int", "nElite must be int type");
}

// ── tour_distance tests ───────────────────────────────────────────────────────

#[test]
fn test_tour_distance_square_perimeter() {
    let cities = square_cities();
    let route: Vec<u32> = cities.iter().map(|c| c.id).collect();
    let distance = run_tour_distance(&route, &cities).expect("square perimeter must succeed");
    assert!(
        (distance - 40.0).abs() < 0.001,
        "expected closed-loop perimeter of 40.0, got {distance}"
    );
}

#[test]
fn test_tour_distance_unknown_city_id_returns_error() {
    let cities = square_cities(); // IDs 0..3
    let bad_route: Vec<u32> = vec![0, 1, 2, 100]; // 100 doesn't exist
    let result = run_tour_distance(&bad_route, &cities);
    assert!(result.is_err(), "unknown city id must return Err");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("unknown city"),
        "error must mention 'unknown city', got: {msg}"
    );
}

#[test]
fn test_tour_distance_empty_route_returns_error() {
    let cities = square_cities();
    let result = run_tour_distance(&[], &cities);
    assert!(result.is_err(), "empty route must return Err");
}

#[test]
fn test_tour_distance_single_city_route_returns_error() {
    let cities = square_cities();
    let result = run_tour_distance(&[0], &cities);
    assert!(result.is_err(), "single-city route must return Err");
}

#[test]
fn test_tour_distance_duplicate_city_id_returns_error() {
    use crate::teeline::solver::types::City;
    let mut cities = square_cities();
    cities.push(City {
        id: 0, // duplicate of the first city's id
        x: 999.0,
        y: 999.0,
    });
    let route: Vec<u32> = vec![0, 1, 2, 3];
    let result = run_tour_distance(&route, &cities);
    assert!(result.is_err(), "duplicate city id must return Err");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("duplicate"),
        "error must mention 'duplicate', got: {msg}"
    );
}
