//! Shared convergence detection for the iterative solvers.
//!
//! A solver that keeps improving should run; one that has stopped improving should be able to say
//! so, because "did this converge, and at which epoch" is not inferable from wall-clock time.

/// Counts consecutive epochs without improvement and reports when a run has plateaued.
///
/// `limit == 0` disables the check, so a solver can opt out without an `Option`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plateau {
    limit: usize,
    stale: usize,
}

impl Plateau {
    pub(crate) fn new(limit: usize) -> Self {
        Plateau { limit, stale: 0 }
    }

    /// Records whether this epoch improved the best-known result, and returns `true` once the run
    /// should stop.
    ///
    /// The return value is the whole point; ignoring it silently disables the feature, so it is
    /// marked `#[must_use]`.
    ///
    /// The count resets on every improvement, so `limit` bounds *consecutive* non-improving
    /// epochs rather than the total — a solver that improves every 400th epoch is not converged at
    /// `limit = 500`.
    #[must_use = "ignoring the result disables the stop condition"]
    pub(crate) fn record(&mut self, improved: bool) -> bool {
        if improved {
            self.stale = 0;
            return false;
        }

        self.stale += 1;
        self.limit > 0 && self.stale >= self.limit
    }

    /// Consecutive non-improving epochs so far, for the convergence log line.
    pub(crate) fn stale_epochs(&self) -> usize {
        self.stale
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_converge_while_improving() {
        let mut p = Plateau::new(3);
        for _ in 0..10 {
            assert!(
                !p.record(true),
                "an improving run must not be called converged"
            );
        }
        assert_eq!(p.stale_epochs(), 0);
    }

    #[test]
    fn converges_after_limit_consecutive_non_improving_epochs() {
        let mut p = Plateau::new(3);
        assert!(!p.record(false));
        assert!(!p.record(false));
        assert!(
            p.record(false),
            "the third stale epoch should converge at limit 3"
        );
        assert_eq!(p.stale_epochs(), 3);
    }

    #[test]
    fn improvement_resets_the_streak() {
        let mut p = Plateau::new(3);
        assert!(!p.record(false));
        assert!(!p.record(false));
        assert!(!p.record(true), "an improvement resets the streak");
        assert_eq!(p.stale_epochs(), 0);
        assert!(!p.record(false));
        assert!(!p.record(false));
        assert!(p.record(false), "three fresh stale epochs should converge");
    }

    /// A solver that improves at least every `limit` epochs never converges: this is what makes the
    /// counter "consecutive" rather than a running total. With `limit = 3` and an improvement every
    /// 3rd epoch, there are never 3 consecutive stale epochs.
    #[test]
    fn periodic_improvement_never_converges() {
        let mut p = Plateau::new(3);
        for epoch in 0..100 {
            assert!(
                !p.record(epoch % 3 == 0),
                "improving every 3rd epoch must not converge at limit 3 (epoch {epoch})"
            );
        }
    }

    /// The boundary that makes the guarantee precise: an improvement slower than the limit does
    /// converge, so "never converges" above is a property of the improvement *rate*, not luck.
    #[test]
    fn a_slower_improvement_than_the_limit_does_converge() {
        let mut p = Plateau::new(3);
        let mut converged_at = None;
        for epoch in 0..100 {
            if p.record(epoch % 4 == 0) {
                converged_at = Some(epoch);
                break;
            }
        }
        assert_eq!(
            converged_at,
            Some(3),
            "epoch 0 improves, so epochs 1-3 are the 3 stale epochs that converge"
        );
    }

    #[test]
    fn limit_zero_disables_the_check() {
        let mut p = Plateau::new(0);
        for _ in 0..1000 {
            assert!(!p.record(false), "limit 0 means the check is disabled");
        }
        assert_eq!(
            p.stale_epochs(),
            1000,
            "the counter still tracks for reporting"
        );
    }
}
