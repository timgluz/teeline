//! Iteration budget and convergence tracking for the iterative solvers.
//!
//! Separated from any one solver so the stopping decision can be tested against a synthetic
//! improvement sequence, rather than by observing a real run of the algorithm.

/// Owns how long a run has been going, and whether it has stopped improving.
///
/// The epoch cap and the convergence threshold live together because they answer the same
/// question — "should this run continue?" — and solvers that kept them apart drifted into
/// different `epochs: 0` meanings and different improvement comparisons.
#[derive(Debug, Clone)]
pub(crate) struct Budget {
    epochs: usize,
    limit: usize,
    epoch: usize,
    stale: usize,
}

impl Budget {
    /// `epochs` is the hard cap, `limit` the convergence threshold; `0` disables convergence.
    pub(crate) fn new(epochs: usize, limit: usize) -> Self {
        Budget {
            epochs,
            limit,
            epoch: 0,
            stale: 0,
        }
    }

    /// Records the epoch that just finished, and returns whether to run another.
    ///
    /// The caller reports whether it *improved* on its own best, because only the caller knows what
    /// "best" means for its algorithm; what this decides is whether progress has stopped. The
    /// non-improving count resets on every improvement, so `limit` bounds *consecutive* stale epochs.
    pub(crate) fn record(&mut self, improved: bool) -> bool {
        if self.epoch >= self.epochs {
            return false;
        }

        self.epoch += 1;
        if improved {
            self.stale = 0;
        } else {
            self.stale += 1;
        }

        !(self.limit > 0 && self.stale >= self.limit)
    }

    /// Epochs completed so far.
    pub(crate) fn epoch(&self) -> usize {
        self.epoch
    }

    /// Consecutive non-improving epochs, for the convergence log line.
    pub(crate) fn stale_epochs(&self) -> usize {
        self.stale
    }

    /// The convergence threshold this run was given.
    pub(crate) fn limit(&self) -> usize {
        self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Improves every epoch: runs to the cap, and never reports convergence.
    #[test]
    fn runs_to_the_cap_while_improving() {
        let mut b = Budget::new(10, 0);
        for _ in 0..10 {
            assert!(b.record(true), "an improving run must continue");
        }
        assert!(!b.record(true), "the cap must stop the run");
        assert_eq!(b.epoch(), 10, "the cap must not be exceeded");
    }

    #[test]
    fn stops_after_the_limit_of_consecutive_non_improving_epochs() {
        let mut b = Budget::new(100_000, 20);
        for _ in 0..19 {
            assert!(b.record(false), "must not stop before the limit");
        }
        assert!(!b.record(false), "the 20th stale epoch must stop the run");
        assert_eq!(b.epoch(), 20);
        assert_eq!(b.stale_epochs(), 20);
    }

    #[test]
    fn improvement_resets_the_streak() {
        let mut b = Budget::new(100, 5);
        for _ in 0..4 {
            assert!(b.record(false));
        }
        assert!(b.record(true), "an improvement resets the streak");
        assert_eq!(b.stale_epochs(), 0);
        assert_eq!(
            b.epoch(),
            5,
            "resetting the streak must not reset the epoch count"
        );
    }

    /// The property the benchmark depends on: a run that keeps finding improvements is never
    /// stopped early, however long it goes on.
    #[test]
    fn periodic_improvement_never_converges_before_the_cap() {
        let mut b = Budget::new(1000, 50);
        for i in 0..1000 {
            // improves every 40th epoch, comfortably inside the 50-epoch limit
            assert!(
                b.record(i % 40 == 0),
                "improvement every 40 epochs must not read as convergence at limit 50"
            );
        }
        assert_eq!(b.epoch(), 1000, "it should reach the cap, not stop early");
    }

    /// An improvement slower than the limit does converge — the boundary that makes the guarantee
    /// above a property of the improvement rate rather than luck.
    #[test]
    fn slower_improvement_than_the_limit_converges() {
        let mut b = Budget::new(1000, 3);
        let mut converged_at = None;
        for i in 0..100 {
            if !b.record(i % 4 == 0) {
                converged_at = Some(b.epoch());
                break;
            }
        }
        assert_eq!(
            converged_at,
            Some(4),
            "epoch 0 improves; epochs 1, 2 and 3 are stale, so the limit is reached on the 4th \
             recorded epoch"
        );
    }

    #[test]
    fn limit_zero_disables_convergence() {
        let mut b = Budget::new(50, 0);
        for _ in 0..50 {
            assert!(b.record(false), "limit 0 must never stop early");
        }
        assert!(!b.record(false), "the epoch cap still applies");
    }

    /// A cap of zero runs nothing. Callers that treat `epochs: 0` as "no cap" must resolve that
    /// before constructing the budget, so the boundary is pinned rather than assumed.
    #[test]
    fn zero_epoch_cap_stops_immediately() {
        let mut b = Budget::new(0, 10);
        assert!(!b.record(true));
        assert_eq!(b.epoch(), 0);
    }
}
