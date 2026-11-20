//! The goal test [`Goal`].

/// A goal test over states of type `S`.
///
/// Takes `&self`: goal tests are pure predicates that need no workspace, which keeps the
/// blanket impl for closures and lets callers pass
/// `&|s: &S| ...` directly (the argument type needs an annotation). Implemented for every
/// `Fn(&S) -> bool`. Algorithms take goals by reference, so no `&G` forwarding impl is
/// needed (and one could not coexist with the closure impl).
///
/// # Examples
///
/// ```
/// use search_core::Goal;
///
/// /// Any state at or beyond a threshold.
/// struct AtLeast(u32);
///
/// impl Goal<u32> for AtLeast {
///     fn is_goal(&self, state: &u32) -> bool {
///         *state >= self.0
///     }
/// }
///
/// fn reached<G: Goal<u32> + ?Sized>(goal: &G, state: u32) -> bool {
///     goal.is_goal(&state)
/// }
///
/// assert!(reached(&AtLeast(5), 7));
/// assert!(!reached(&|s: &u32| *s == 3, 7));
/// ```
pub trait Goal<S> {
    /// Returns `true` if `state` is a goal state.
    fn is_goal(&self, state: &S) -> bool;
}

impl<S, F: Fn(&S) -> bool> Goal<S> for F {
    fn is_goal(&self, state: &S) -> bool {
        self(state)
    }
}
