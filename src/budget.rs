//! Early stopping and time slicing: the [`Budget`] trait over a crate's own statistics type,
//! the never-exhausted budget [`Unlimited`], and the [`Status`] a sliced search reports.

/// Decides when a search must stop before it finishes, given its counters so far.
///
/// `T` is the statistics type of the algorithm, such as a struct with `expanded` and
/// `generated` counters. Each search crate defines its own statistics and limits (the
/// counters differ between graph search and MDP solvers) and implements `Budget<Stats>` for
/// them; this trait lets budget adapters, such as a wall-clock deadline in a `std` crate, be
/// written once for every algorithm.
///
/// Algorithms ask the budget before each unit of work, so a limit of `n` stops after exactly
/// `n` units. In a time-sliced search the counters start from zero in each start or resume
/// call, so a budget is the allowance of one slice (see [`Status`]). Budgets are taken by
/// value; to keep a struct budget's state across slices, pass a closure that borrows it:
/// `&mut |s: &T| limits.exhausted(s)`.
///
/// Implemented for every `FnMut(&T) -> bool`; annotate the closure argument
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

/// A budget that is never exhausted: the search runs until it finishes.
///
/// # Examples
///
/// ```
/// use search_core::{Budget, Unlimited};
///
/// assert!(!Unlimited.exhausted(&u64::MAX));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Unlimited;

impl<T> Budget<T> for Unlimited {
    fn exhausted(&mut self, _stats: &T) -> bool {
        false
    }
}

/// How one call of a time-sliced search ended: it finished, or its [`Budget`] stopped it.
///
/// A search that can be spread over several frames returns this from its start and resume
/// calls. After [`Status::OutOfBudget`], the search keeps its state in its workspace, and
/// the crate's resume call continues where it stopped. The resume call itself is
/// crate-specific, since what a search needs to continue differs between algorithms.
///
/// Non-exhaustive, so that searches can gain further outcomes without a breaking change;
/// matches outside this crate need a `_` arm.
///
/// # Examples
///
/// A countdown sliced over two frames, then run to the end:
///
/// ```
/// use search_core::{Budget, Status, Unlimited};
///
/// /// A resumable search: counts `left` down to zero, one unit of work per step.
/// struct Countdown {
///     left: u32,
/// }
///
/// impl Countdown {
///     /// Continues the run. The budget sees the units done in this call only.
///     fn resume(&mut self, mut budget: impl Budget<u32>) -> Status {
///         let mut done = 0;
///         while self.left > 0 {
///             if budget.exhausted(&done) {
///                 return Status::OutOfBudget;
///             }
///
///             self.left -= 1;
///             done += 1;
///         }
///
///         Status::Finished
///     }
/// }
///
/// let mut search = Countdown { left: 5 };
/// let mut frames = 0;
/// let mut per_frame = |done: &u32| {
///     frames += 1;
///     *done >= 2
/// };
///
/// assert_eq!(search.resume(&mut per_frame), Status::OutOfBudget);
/// assert_eq!(search.left, 3);
/// assert_eq!(search.resume(&mut per_frame), Status::OutOfBudget);
/// assert_eq!(search.left, 1);
/// assert_eq!(search.resume(Unlimited), Status::Finished);
/// assert_eq!(frames, 6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Status {
    /// The search did all its work.
    Finished,
    /// The budget stopped the search before it finished; resuming continues it.
    OutOfBudget,
}
