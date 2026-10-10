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

    /// Decides whether the next epoch may run, given whether the previous one improved.
    ///
    /// Called at the top of the loop. `improved` describes the epoch that just finished, so it is
    /// ignored on the first call — nothing has run yet, and counting it would charge the solver for
    /// an epoch it never took. That first call is what admits epoch 0.
    ///
    /// Returns `false` once the epoch cap is reached, or once the run has stalled for `limit`
    /// consecutive epochs. The non-improving count resets on every improvement, so `limit` bounds
    /// *consecutive* stale epochs rather than a total.
    pub(crate) fn record(&mut self, improved: bool) -> bool {
        if self.epoch >= self.epochs {
            return false;
        }

        // The first call admits epoch 0, so there is no previous result to count yet. Counting it
        // would both shorten the run by one epoch and charge the solver for work it never did.
        if self.epoch == 0 {
            self.epoch = 1;
            return true;
        }

        if improved {
            self.stale = 0;
        } else {
            self.stale += 1;
        }

        // Checked after counting: the epoch that reaches the limit is the last one admitted, and the
        // call that would have started the next is where the run ends.
        if self.converged() {
            return false;
        }

        self.epoch += 1;
        true
    }

    /// Epochs completed. Also the number of the epoch that ran last, since they are one-based.
    pub(crate) fn epoch(&self) -> usize {
        self.epoch
    }

    /// Zero-based index of the epoch currently running — what a `for epoch in 0..n` loop would have
    /// bound. Use this for anything that feeds the epoch into a schedule, so a solver's first epoch
    /// sees `0` as it did before.
    pub(crate) fn index(&self) -> usize {
        self.epoch.saturating_sub(1)
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

    /// Drives a budget the way every solver does: `record` is called at the top of the loop with the
    /// *previous* epoch's outcome, and the first call admits epoch 0 with `improved` ignored. Feeding
    /// it a sequence of per-epoch outcomes keeps the tests honest about that call pattern, which an
    /// earlier version of these tests got wrong and so missed an off-by-one.
    fn run(cap: usize, limit: usize, outcomes: &[bool]) -> (usize, usize, bool, Vec<usize>) {
        let mut b = Budget::new(cap, limit);
        let mut seen = Vec::new();
        let mut previous = true; // admits the first epoch

        for &outcome in outcomes {
            if !b.record(previous) {
                return (b.epoch(), b.stale_epochs(), b.converged(), seen);
            }
            seen.push(b.index());
            previous = outcome;
        }
        (b.epoch(), b.stale_epochs(), b.converged(), seen)
    }

    /// The index a solver sees is zero-based, exactly as `for epoch in 0..n` gave it. Schedules
    /// derived from the epoch (PSO's inertia, GSA's gravitational constant) depend on this.
    #[test]
    fn index_is_zero_based() {
        let (_, _, _, seen) = run(5, 0, &[true, true, true, true, true]);
        assert_eq!(seen, vec![0, 1, 2, 3, 4], "the body must see 0, 1, 2, ...");
    }

    #[test]
    fn runs_to_the_cap_while_improving() {
        let (epochs, stale, converged, seen) = run(10, 0, &[true; 20]);
        assert_eq!(epochs, 10, "the cap must bound the run");
        assert_eq!(stale, 0, "an improving run never goes stale");
        assert!(!converged, "the cap, not convergence, ended it");
        assert_eq!(seen.len(), 10);
    }

    /// `limit` consecutive stale epochs end the run, and the count is exact: 20 stale epochs means
    /// 20 epochs run, with the limit deciding on the call that would have started the 21st.
    #[test]
    fn stops_after_the_limit_of_consecutive_non_improving_epochs() {
        let (epochs, stale, converged, _) = run(100_000, 20, &[false; 100]);
        assert_eq!(epochs, 20, "exactly the limit, not one more");
        assert_eq!(stale, 20);
        assert!(converged, "the limit is what ended this run");
    }

    /// The first call admits an epoch without counting it as stale, even though it passes `true`.
    #[test]
    fn the_first_call_does_not_count_as_an_epoch_result() {
        // A limit of 1 stops after exactly one stale epoch; if the first admission counted as a
        // result, the run would stop before running anything.
        let (epochs, stale, converged, seen) = run(100, 1, &[false, false]);
        assert_eq!(seen, vec![0], "only epoch 0 runs");
        assert_eq!(epochs, 1);
        assert_eq!(stale, 1);
        assert!(converged);
    }

    #[test]
    fn improvement_resets_the_streak() {
        // epoch 0, two stale epochs, an improvement, then three more stale epochs
        let (epochs, stale, converged, _) =
            run(100, 3, &[false, false, true, false, false, false, false]);
        assert!(
            converged,
            "three stale epochs after the improvement end the run"
        );
        assert_eq!(
            stale, 3,
            "the streak ran again from zero after the improvement"
        );
        // epochs 0-5 ran; the call that would have started the 7th found the limit reached
        assert_eq!(
            epochs, 6,
            "clearing the streak must not reset the epoch count"
        );
    }

    /// A run improving at least every `limit` epochs is never mistaken for converged.
    #[test]
    fn periodic_improvement_never_converges_before_the_cap() {
        let outcomes: Vec<bool> = (0..1000).map(|i| i % 40 == 0).collect();
        let (epochs, _, converged, _) = run(1000, 50, &outcomes);
        assert_eq!(epochs, 1000, "it should reach the cap, not stop early");
        assert!(!converged);
    }

    /// The boundary: improving every 4th epoch with a limit of 3 does converge, which is what makes
    /// the guarantee above a property of the improvement rate rather than luck.
    #[test]
    fn slower_improvement_than_the_limit_converges() {
        let outcomes: Vec<bool> = (0..100).map(|i| i % 4 == 0).collect();
        let (_, _, converged, _) = run(1000, 3, &outcomes);
        assert!(
            converged,
            "improving only every 4th epoch must converge at limit 3"
        );
    }

    #[test]
    fn limit_zero_disables_convergence() {
        let (epochs, _, converged, _) = run(50, 0, &[false; 100]);
        assert_eq!(epochs, 50, "limit 0 must never stop early");
        assert!(!converged);
    }

    /// Reaching the cap with a stale streak shorter than the limit must not be reported as
    /// convergence — the run ended because it ran out of epochs.
    #[test]
    fn cap_reached_with_a_stale_streak_is_not_convergence() {
        let (epochs, stale, converged, _) = run(5, 50, &[false; 100]);
        assert_eq!(epochs, 5);
        assert_eq!(stale, 4, "the first call contributes no outcome");
        assert!(!converged);
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
