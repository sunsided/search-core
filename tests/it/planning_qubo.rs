//! Plays `planning-qubo`: a sampler-like search that is sliced over frames with the shared
//! `Budget`, `Status` and `Unlimited`, while its run state stays in its own workspace.

use search_core::{Budget, Status, Unlimited};

/// The work counters of one call, as `planning-qubo` keeps them. Each call starts from zero.
#[derive(Debug, Default, PartialEq, Eq)]
struct Stats {
    /// Steps taken in this call.
    steps: u64,
}

/// A resumable search: walks `0..len` and keeps the best (smallest) `value(i)`.
struct Walker {
    /// Number of states to visit in a run.
    len: u64,
    /// The next state to visit.
    next: u64,
    /// The best value found so far.
    best: u64,
}

impl Walker {
    /// Starts a new run that visits `len` states, replacing any earlier run.
    fn start(len: u64, budget: impl Budget<Stats>) -> (Self, Status, Stats) {
        let mut walker = Self {
            len,
            next: 0,
            best: u64::MAX,
        };
        let (status, work) = walker.resume(budget);
        (walker, status, work)
    }

    /// Continues the run until it finishes or `budget` is exhausted.
    fn resume(&mut self, mut budget: impl Budget<Stats>) -> (Status, Stats) {
        let mut work = Stats::default();
        while self.next < self.len {
            if budget.exhausted(&work) {
                return (Status::OutOfBudget, work);
            }

            self.best = self.best.min(value(self.next));
            self.next += 1;
            work.steps += 1;
        }

        (Status::Finished, work)
    }
}

/// The objective of the toy search, with its minimum `0` at `7`.
fn value(i: u64) -> u64 {
    i.abs_diff(7)
}

/// A downstream match: `Status` is non-exhaustive, so a `_` arm is required.
fn describe(status: Status) -> &'static str {
    match status {
        Status::Finished => "finished",
        Status::OutOfBudget => "paused",
        _ => "other",
    }
}

#[test]
fn a_budget_of_n_stops_after_exactly_n_steps_per_call() {
    let per_frame = |work: &Stats| work.steps >= 4;
    let (mut walker, status, work) = Walker::start(10, per_frame);
    assert_eq!(
        (describe(status), work.steps, walker.next),
        ("paused", 4, 4)
    );

    let (status, work) = walker.resume(per_frame);
    assert_eq!(
        (describe(status), work.steps, walker.next),
        ("paused", 4, 8)
    );
    assert_eq!(walker.best, 0);

    let (status, work) = walker.resume(per_frame);
    assert_eq!(
        (describe(status), work.steps, walker.next),
        ("finished", 2, 10)
    );
}

#[test]
fn a_closure_budget_keeps_its_state_across_slices_by_mut_reference() {
    let mut checks = 0_u32;
    let mut counting = |work: &Stats| {
        checks += 1;
        work.steps >= 3
    };

    let (mut walker, status, _) = Walker::start(5, &mut counting);
    assert_eq!(status, Status::OutOfBudget);
    let (status, _) = walker.resume(&mut counting);
    assert_eq!(status, Status::Finished);
    assert_eq!(checks, 4 + 2);
}

#[test]
fn unlimited_runs_to_the_end_and_resuming_a_finished_run_does_no_work() {
    let (mut walker, status, work) = Walker::start(10, Unlimited);
    assert_eq!((status, work.steps, walker.best), (Status::Finished, 10, 0));

    let (status, work) = walker.resume(Unlimited);
    assert_eq!((status, work), (Status::Finished, Stats::default()));
}
