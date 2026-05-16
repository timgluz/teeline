use wasmtime::component::{Component, Linker};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtxBuilder, WasiView};

wasmtime::component::bindgen!({
    world: "solver",
    path: "teeline-wasm/wit",
});

struct HostState {
    table: wasmtime_wasi::ResourceTable,
    wasi: wasmtime_wasi::WasiCtx,
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut wasmtime_wasi::ResourceTable {
        &mut self.table
    }
    fn ctx(&mut self) -> &mut wasmtime_wasi::WasiCtx {
        &mut self.wasi
    }
}

fn make_engine() -> Engine {
    let mut config = Config::new();
    config.wasm_component_model(true);
    Engine::new(&config).unwrap()
}

fn load_component(engine: &Engine) -> Component {
    let path = "target/wasm32-wasip2/debug/teeline_wasm.wasm";
    Component::from_file(engine, path).expect(
        "WASM component not found — run: cargo component build --manifest-path teeline-wasm/Cargo.toml",
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
        City { id: 0, x: 565.0, y: 575.0 },
        City { id: 1, x: 25.0, y: 185.0 },
        City { id: 2, x: 345.0, y: 750.0 },
        City { id: 3, x: 945.0, y: 685.0 },
        City { id: 4, x: 845.0, y: 655.0 },
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
    let expected: Vec<u32> = (0..n_cities as u32).collect();
    assert_eq!(sorted, expected, "each city must be visited exactly once");
}

fn run_solver(solver_name: &str) {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker).unwrap();
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
fn test_tabu_search() {
    run_solver("tabu_search")
}
#[test]
fn test_stochastic_hill() {
    run_solver("stochastic_hill")
}

#[test]
fn unknown_solver_returns_err() {
    let engine = make_engine();
    let component = load_component(&engine);
    let mut linker: Linker<HostState> = Linker::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker).unwrap();
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
