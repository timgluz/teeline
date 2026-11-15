//! Distance-matrix construction microbenchmarks.
//!
//! Run with: `cargo bench --bench distance_matrix`
//!
//! Covers the eager O(n²) build path at the tracked fixture sizes (52/280/532) and the four
//! coordinate distance primitives. The 25k+ memory scaling measurements live in
//! `scripts/bench-distance-scaling.sh`, not here: criterion keeps inputs resident and would
//! report a peak that includes the previous iteration's allocations.

use std::path::Path;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use teeline::tsp::distance_matrix::DistanceMatrix;
use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::{DistanceSource, DistanceType, tsplib};

const DATASETS: &[&str] = &["berlin52", "a280", "att532"];

fn load_cities(name: &str) -> Vec<KDPoint> {
    // tests/fixtures/ only: it is tracked, so this works on a fresh checkout.
    let path = Path::new("tests/fixtures").join(format!("{name}.tsp"));
    assert!(
        path.exists(),
        "TSPLIB fixture {name}.tsp not found; run from the repo root"
    );
    let data =
        tsplib::read_from_file(&path).unwrap_or_else(|e| panic!("failed to load {path:?}: {e}"));
    data.cities().to_vec()
}

fn bench_build_euc_2d(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_euc_2d");
    for name in DATASETS {
        let cities = load_cities(name);
        group.bench_with_input(BenchmarkId::from_parameter(name), &cities, |b, cities| {
            b.iter(|| {
                black_box(DistanceMatrix::build(cities, DistanceType::Euc2D).unwrap());
            });
        });
    }
    group.finish();
}

fn bench_build_primitives(c: &mut Criterion) {
    let cities = load_cities("berlin52");
    let mut group = c.benchmark_group("build_primitive");
    for dt in [
        DistanceType::Euc2D,
        DistanceType::Geo,
        DistanceType::Att,
        DistanceType::Ceil2D,
    ] {
        group.bench_with_input(
            BenchmarkId::from_parameter(dt.as_str()),
            &cities,
            |b, cities| {
                b.iter(|| {
                    black_box(DistanceMatrix::build(cities, dt).unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_lookup(c: &mut Criterion) {
    let cities = load_cities("att532");
    let dm = DistanceMatrix::build(&cities, DistanceType::Euc2D).unwrap();
    let last = dm.num_cities();
    c.bench_function("distance_between_last_pair", |b| {
        b.iter(|| black_box(dm.distance_between(1, last).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_build_euc_2d,
    bench_build_primitives,
    bench_lookup
);
criterion_main!(benches);
