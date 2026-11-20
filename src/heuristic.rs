//! Cost-to-go estimates: the [`Heuristic`] trait, [`heuristic_fn`] for closures and the blind
//! heuristic [`Zero`].

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::cost::Cost;

/// An estimate of the cost from a state of type `S` to the nearest goal, in cost type `C`.
///
/// `estimate` takes `&mut self`, so a heuristic can keep a reusable workspace (scratch
/// buffers, caches) without interior mutability. Searches take the heuristic by value and own
/// it for the run; pass `&mut h` to keep `h` and its workspace afterwards. Share read-only
/// tables by reference and create one heuristic per thread.
///
/// `C` defaults to `u32` and is deliberately unbounded: scalar searches use a [`Cost`],
/// multi-objective searches a cost vector such as `[u32; 2]`, and probabilistic planners an
/// expected cost such as `f64`. Each algorithm states what it requires (admissibility,
/// consistency).
///
/// Implemented for `&mut H` and (feature `alloc`) `Box<H>` of any heuristic `H`, so
/// `Box<dyn Heuristic<S, C>>` works; for [`Zero`]; and for closures through
/// [`heuristic_fn`]. A blanket impl for closures cannot coexist with the one for `&mut H`,
/// because `&mut F` is itself a closure (E0119).
///
/// # Examples
///
/// A heuristic with a workspace:
///
/// ```
/// use search_core::Heuristic;
///
/// /// Counts how often it was asked; a real heuristic would reuse buffers here.
/// struct Distance {
///     target: u32,
///     calls: usize,
/// }
///
/// impl Heuristic<u32> for Distance {
///     fn estimate(&mut self, state: &u32) -> Option<u32> {
///         self.calls += 1;
///         self.target.checked_sub(*state)
///     }
/// }
///
/// let mut h = Distance { target: 10, calls: 0 };
/// assert_eq!(h.estimate(&4), Some(6));
/// assert_eq!((&mut h).estimate(&11), None);
/// assert_eq!(h.calls, 2);
/// ```
pub trait Heuristic<S, C = u32> {
    /// Estimates the remaining cost from `state` to a goal.
    ///
    /// `None` means no goal is reachable from `state` (a proven dead end). Searches drop such
    /// states, so a heuristic must only return `None` when that is certain.
    fn estimate(&mut self, state: &S) -> Option<C>;
}

impl<S, C, H: Heuristic<S, C> + ?Sized> Heuristic<S, C> for &mut H {
    fn estimate(&mut self, state: &S) -> Option<C> {
        (**self).estimate(state)
    }
}

#[cfg(feature = "alloc")]
impl<S, C, H: Heuristic<S, C> + ?Sized> Heuristic<S, C> for Box<H> {
    fn estimate(&mut self, state: &S) -> Option<C> {
        (**self).estimate(state)
    }
}

/// Turns a closure `FnMut(&S) -> Option<C>` into a [`Heuristic`].
///
/// # Examples
///
/// ```
/// use search_core::{Heuristic, heuristic_fn};
///
/// let mut calls = 0;
/// let mut h = heuristic_fn(|n: &u32| {
///     calls += 1;
///     10_u32.checked_sub(*n)
/// });
///
/// assert_eq!(h.estimate(&3), Some(7));
/// assert_eq!(h.estimate(&12), None);
/// drop(h);
/// assert_eq!(calls, 2);
/// ```
pub fn heuristic_fn<S, C, F: FnMut(&S) -> Option<C>>(f: F) -> HeuristicFn<F> {
    HeuristicFn(f)
}

/// A closure used as a [`Heuristic`]; created by [`heuristic_fn`].
#[derive(Debug, Clone, Copy)]
pub struct HeuristicFn<F>(F);

impl<S, C, F: FnMut(&S) -> Option<C>> Heuristic<S, C> for HeuristicFn<F> {
    fn estimate(&mut self, state: &S) -> Option<C> {
        (self.0)(state)
    }
}

/// The blind heuristic: estimates [`Cost::ZERO`] everywhere. Turns A* into Dijkstra's
/// algorithm and is admissible and consistent for every non-negative cost.
///
/// Implemented for every scalar [`Cost`] only. Cost vectors and raw floats are not `Cost`;
/// crates that use them keep their own zero heuristic (see `MIGRATION.md`).
///
/// # Examples
///
/// ```
/// use search_core::{Decimal, Heuristic, Zero};
///
/// assert_eq!(Heuristic::<&str>::estimate(&mut Zero, &"any"), Some(0_u32));
/// assert_eq!(Heuristic::<u8, Decimal<2>>::estimate(&mut Zero, &1), Some(Decimal::from_raw(0)));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Zero;

impl<S, C: Cost> Heuristic<S, C> for Zero {
    fn estimate(&mut self, _state: &S) -> Option<C> {
        Some(C::ZERO)
    }
}
