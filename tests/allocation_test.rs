//! Allocation gate for the distance-source hot loops.
//!
//! Positions are what solvers touch per candidate move, so a hidden allocation in
//! `distance_by_pos`, `pos2city_id`/`city_id2pos`, or `tour_length` would show up as
//! allocator traffic in every local-search pass. This binary installs a counting global
//! allocator and asserts those loops stay allocation-free on both `IdIndex` branches.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use teeline::tsp::distance_matrix::DistanceMatrix;
use teeline::tsp::kdtree::KDPoint;
use teeline::tsp::{DistanceSource, DistanceType, Position};

/// Counting allocator used only by this test binary.
///
/// `GlobalAlloc` is an unsafe trait, but this is the only way to observe allocations
/// without an external dependency. Every method forwards to `System` unchanged; the
/// atomics are lock-free, so the counting path cannot itself allocate or recurse.
struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn source(ids: impl Iterator<Item = usize>) -> DistanceMatrix {
    let cities: Vec<KDPoint> = ids
        .enumerate()
        .map(|(i, id)| KDPoint::new_with_id(id, &[i as f32, 0.0]))
        .collect();
    DistanceMatrix::build(&cities, DistanceType::Euc2D).expect("unique city ids")
}

fn assert_hot_loops_allocate_nothing(source: &DistanceMatrix, label: &str) {
    let n = source.num_cities();
    let tour_ids: Vec<usize> = (0..n)
        .map(|pos| source.pos2city_id(Position::new(pos)).unwrap())
        .collect();
    let tour_positions: Vec<Position> = (0..n).map(Position::new).collect();

    // Warm up the trait dispatch and any lazy state before the measured window.
    let _ = source.distance_between(tour_ids[0], tour_ids[1]).unwrap();
    let _ = source.tour_length(&tour_ids);
    let _ = source.tour_length_by_pos(&tour_positions);

    ALLOCATIONS.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);

    let mut checksum = 0.0f32;
    for i in 0..10_000 {
        let a = Position::new(i % n);
        let b = Position::new((i + 1) % n);

        checksum += source.distance_by_pos(a, b).unwrap();
        checksum += source
            .distance_between(tour_ids[i % n], tour_ids[(i + 1) % n])
            .unwrap();

        let id = source.pos2city_id(a).unwrap();
        let pos = source.city_id2pos(id).unwrap();
        checksum += pos.into_inner() as f32;
    }
    for _ in 0..1_000 {
        checksum += source.tour_length(&tour_ids);
        checksum += source.tour_length_by_pos(&tour_positions);
    }

    COUNTING.store(false, Ordering::SeqCst);

    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    assert_eq!(
        allocations, 0,
        "{label}: hot-loop lookups allocated {allocations} time(s)"
    );
    assert!(checksum.is_finite(), "{label}: checksum must stay finite");
}

#[test]
fn distance_source_hot_loops_do_not_allocate() {
    // Dense `IdIndex` branch (TSPLIB-style 1-based ids).
    assert_hot_loops_allocate_nothing(&source(1..=64), "dense ids");
    // Sparse `IdIndex` branch (max id far above 4 * n).
    assert_hot_loops_allocate_nothing(&source((1..=64).map(|i| i * 1_000)), "sparse ids");
}
