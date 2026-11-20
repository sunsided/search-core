//! Early stopping: the [`Budget`] trait over a crate's own statistics type.

/// Decides when a search must stop before it finishes, given its counters so far.
///
/// `T` is the statistics type of the algorithm, such as a struct with `expanded` and
/// `generated` counters. Each search crate defines its own statistics and limits (the
/// counters differ between graph search and MDP solvers) and implements `Budget<Stats>` for
/// them; this trait lets budget adapters, such as a wall-clock deadline in a `std` crate, be
/// written once for every algorithm.
///
/// Algorithms ask the budget before each unit of work, so a limit of `n` stops after exactly
/// `n` units. Implemented for every `FnMut(&T) -> bool`; annotate the closure argument
/// (`|stats: &Stats| ...`), since the type cannot be inferred through the blanket impl.
///
/// # Examples
///
/// ```
/// use search_core::Budget;
///
/// /// The counters of some search.
/// struct Stats {
///     expanded: u64,
/// }
///
/// /// A search crate's limits.
/// struct Limits {
///     max_expansions: Option<u64>,
/// }
///
/// impl Budget<Stats> for Limits {
///     fn exhausted(&mut self, stats: &Stats) -> bool {
///         self.max_expansions.is_some_and(|max| stats.expanded >= max)
///     }
/// }
///
/// fn stops(mut budget: impl Budget<Stats>, expanded: u64) -> bool {
///     budget.exhausted(&Stats { expanded })
/// }
///
/// assert!(stops(Limits { max_expansions: Some(3) }, 3));
/// assert!(!stops(Limits { max_expansions: None }, 3));
/// assert!(stops(|stats: &Stats| stats.expanded > 1, 2));
/// ```
pub trait Budget<T> {
    /// Returns `true` if the search must stop now, given the counters so far.
    fn exhausted(&mut self, stats: &T) -> bool;
}

impl<T, F: FnMut(&T) -> bool> Budget<T> for F {
    fn exhausted(&mut self, stats: &T) -> bool {
        self(stats)
    }
}
