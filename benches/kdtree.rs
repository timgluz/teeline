//! KD-tree microbenchmarks.
//!
//! Run with: `cargo bench --bench kdtree`
//!
//! Three groups, each parameterized over `berlin52` (n=52), `a280` (n=280),
//! and `att532` (n=532):
//!
//! - `build`            — `kdtree::from_cities(&cities)` (tree construction)
//! - `nearest_k5`       — pre-built tree, one `nearest(city, 5)` query per city
//! - `build_candidates` — end-to-end mirror of `lin_kernighan::build_candidates`
//!   (build tree + N queries with k=5), the primary hot path
//!
//! TSPLIB files live under `data/tsplib/` and must be present; run from the
//! repo root.

use std::path::Path;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use teeline::tsp::kdtree::{self, KDPoint};
use teeline::tsp::tsplib;

const DATASETS: &[&str] = &["berlin52", "a280", "att532"];
const K_NEAREST: usize = 5;

fn load_cities(name: &str) -> Vec<KDPoint> {
    // Prefer tests/fixtures/ (tracked in git, always available) over data/tsplib/
    // (gitignored, populated by download_data.sh).
    let candidates = [
        Path::new("tests/fixtures").join(format!("{name}.tsp")),
        Path::new("data/tsplib").join(format!("{name}.tsp")),
    ];
    let path = candidates.iter().find(|p| p.exists()).unwrap_or_else(|| {
        panic!(
            "TSPLIB file {name}.tsp not found in tests/fixtures/ or data/tsplib/. \
                 Run from repo root."
        )
    });
    let data =
        tsplib::read_from_file(path).unwrap_or_else(|e| panic!("failed to load {path:?}: {e}"));
    data.cities().to_vec()
}

/// Tree construction only — `from_cities` + median partition.
fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("build");
    for name in DATASETS {
        let cities = load_cities(name);
        group.bench_with_input(BenchmarkId::from_parameter(name), &cities, |b, cities| {
            b.iter(|| black_box(kdtree::from_cities(black_box(cities))));
        });
    }
    group.finish();
}

/// Single-query latency: pre-built tree, one `nearest(city, 5)` per city.
/// Isolates the query path from the build path.
fn bench_nearest_k5(c: &mut Criterion) {
    let mut group = c.benchmark_group("nearest_k5");
    for name in DATASETS {
        let cities = load_cities(name);
        let tree = kdtree::from_cities(&cities);
        group.bench_with_input(BenchmarkId::from_parameter(name), &cities, |b, cities| {
            b.iter(|| {
                let mut checksum = 0usize;
                for city in cities {
                    let res = tree.nearest(black_box(city), K_NEAREST);
                    checksum = checksum.wrapping_add(res.point.id);
                }
                black_box(checksum);
            });
        });
    }
    group.finish();
}

/// End-to-end mirror of `lin_kernighan::build_candidates` (build tree + N
/// queries with k=5). The realistic hot path that motivates the perf work.
fn bench_build_candidates(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_candidates");
    for name in DATASETS {
        let cities = load_cities(name);
        group.bench_with_input(BenchmarkId::from_parameter(name), &cities, |b, cities| {
            b.iter(|| {
                let tree = kdtree::from_cities(cities);
                let mut checksum = 0usize;
                for city in cities {
                    let res = tree.nearest(city, K_NEAREST);
                    for item in res.nearest() {
                        checksum = checksum.wrapping_add(item.point.id);
                    }
                }
                black_box(checksum);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_build,
    bench_nearest_k5,
    bench_build_candidates
);
criterion_main!(benches);
