//! Plays `stochastic-search`: `f64` expected-cost heuristics (not a [`Cost`]), a local zero
//! heuristic for `f64`, and a budget over MDP counters.

use search_core::{Budget, Goal, Heuristic, Zero, heuristic_fn};

/// The counters of an MDP solver run.
#[derive(Default)]
struct Stats {
    /// Bellman backups performed.
    backups: u64,
    /// Distinct states stored.
    states: u64,
}

/// Finite default limits, as `stochastic-search` keeps them.
struct Limits {
    /// Stop after this many backups. `None` is unbounded.
    max_backups: Option<u64>,
    /// Stop once this many states are stored. `None` is unbounded.
    max_states: Option<u64>,
}

impl Budget<Stats> for Limits {
    fn exhausted(&mut self, stats: &Stats) -> bool {
        self.max_backups.is_some_and(|max| stats.backups >= max)
            || self.max_states.is_some_and(|max| stats.states >= max)
    }
}

/// Runs a fake solver loop: one backup and one stored state per step until the budget stops.
fn run(
    mut heuristic: impl Heuristic<u8, f64>,
    goal: &impl Goal<u8>,
    mut budget: impl Budget<Stats>,
) -> (u64, f64) {
    let mut stats = Stats::default();
    let mut value = 0.0;
    let mut current = 0_u8;

    while !goal.is_goal(&current) && !budget.exhausted(&stats) {
        value += heuristic.estimate(&current).unwrap_or(f64::INFINITY);
        stats.backups += 1;
        stats.states += 1;
        current += 1;
    }

    (stats.backups, value)
}

/// Both limits, closures and `f64` heuristics compose.
#[test]
fn mdp_shapes_compile() {
    let limits = |backups, states| Limits {
        max_backups: backups,
        max_states: states,
    };

    assert_eq!(
        run(Zero, &|s: &u8| *s == 9, limits(Some(4), None)),
        (4, 0.0)
    );
    assert_eq!(
        run(Zero, &|s: &u8| *s == 9, limits(None, Some(2))),
        (2, 0.0)
    );
    assert_eq!(
        run(
            heuristic_fn(|s: &u8| Some(f64::from(*s))),
            &|s: &u8| *s == 3,
            |_: &Stats| false
        ),
        (3, 3.0)
    );
}
