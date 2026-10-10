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

    /// Records whether the epoch about to run may proceed, and returns whether it should.
    ///
    /// The caller reports whether it *improved* on its own best, because only the caller knows what
    /// "best" means for its algorithm; what this decides is whether progress has stopped. The
    /// non-improving count resets on every improvement, so `limit` bounds *consecutive* stale epochs.
    ///
    /// Called at the top of the loop, so it only counts an epoch it has actually admitted — the count
    /// is the number of epochs run, not the number requested.
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

        // Checked after counting, so the epoch that reaches the limit is the last one admitted and
        // the count stays exact — checking first would require an extra call to notice.
        !self.converged()
    }

    /// Epochs run so far.
    pub(crate) fn epoch(&self) -> usize {
        self.epoch
    }

    /// Whether convergence — not the epoch cap — is what ended the run.
    ///
    /// Callers report this rather than re-deriving it: a run can reach its cap while also having a
    /// stale streak at the limit, and reporting that as convergence would misattribute the stop.
    pub(crate) fn converged(&self) -> bool {
        self.limit > 0 && self.stale >= self.limit
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

    /// Reaching the cap with a stale streak still at the limit must NOT be reported as convergence —
    /// the run ended because it ran out of epochs, and logging it as convergence would misattribute
    /// the stop.
    #[test]
    fn cap_reached_with_a_stale_streak_is_not_convergence() {
        let mut b = Budget::new(5, 50);
        for _ in 0..5 {
            assert!(b.record(false), "5 epochs fit inside the cap and the limit");
        }
        assert!(!b.record(false), "the cap must end the run");
        assert!(
            !b.converged(),
            "a stale streak below the limit is not convergence, even at the cap"
        );
        assert_eq!(b.epoch(), 5);
    }

    #[test]
    fn convergence_is_reported_only_when_the_limit_ended_the_run() {
        let mut b = Budget::new(1000, 3);
        assert!(b.record(true));
        assert!(b.record(false));
        assert!(b.record(false));
        assert!(!b.record(false), "the third stale epoch ends the run");
        assert!(b.converged(), "the limit, not the cap, is what stopped it");
        assert_eq!(
            b.epoch(),
            4,
            "the count includes the epoch that reached the limit"
        );
    }

    /// `epoch()` counts epochs admitted, so a run stopped by convergence reports the epochs it
    /// actually ran rather than one more.
    #[test]
    fn epoch_counts_only_epochs_that_ran() {
        let mut b = Budget::new(100, 2);
        assert!(b.record(false));
        assert!(!b.record(false), "the second stale epoch ends the run");
        assert_eq!(b.epoch(), 2, "two epochs ran, not three");
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
