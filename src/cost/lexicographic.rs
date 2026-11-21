//! The pair cost [`Lexicographic`]: a primary cost with a secondary tie-breaker.

use core::ops::Add;

use super::Cost;

/// A path cost ordered by [`primary`](Self::primary) first, with ties broken by
/// [`secondary`](Self::secondary). Both components add up along a path.
///
/// Use it to break ties between equally cheap paths by a second criterion, such as the
/// fewest steps and then the fewest turns, or the cheapest route and then the fewest
/// conflicts with other agents. Any search generic over [`Cost`] accepts it unchanged.
///
/// # Heuristics
///
/// A heuristic `(h, S::ZERO)`, where `h` is admissible or consistent for the primary cost,
/// keeps that property for the pair: `(h(u), 0) <= (c + h(v), s)` whenever
/// `h(u) <= c + h(v)`. A* then returns a path of minimal primary cost and, among those, of
/// minimal secondary cost.
///
/// # Overflow
///
/// `+` adds component-wise and overflows like its components. [`Cost::saturating_add`] and
/// [`Cost::saturating_times`] saturate each component separately.
///
/// # Examples
///
/// ```
/// use search_core::{Cost, Lexicographic};
///
/// let fast = Lexicographic::new(10_u32, 3_u32);
/// let calm = Lexicographic::new(10_u32, 1_u32);
/// let slow = Lexicographic::new(11_u32, 0_u32);
///
/// assert!(calm < fast); // Same primary cost: fewer conflicts win.
/// assert!(fast < slow); // The primary cost always decides first.
/// assert_eq!(fast + calm, Lexicographic::new(20, 4));
/// assert_eq!(Lexicographic::<u32, u32>::ZERO, Lexicographic::new(0, 0));
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lexicographic<P, S> {
    /// The cost that is minimized first.
    pub primary: P,
    /// The tie-breaker, compared only between equal primary costs.
    pub secondary: S,
}

impl<P, S> Lexicographic<P, S> {
    /// Creates the cost `(primary, secondary)`.
    #[must_use]
    pub const fn new(primary: P, secondary: S) -> Self {
        Self { primary, secondary }
    }
}

impl<P: Add<Output = P>, S: Add<Output = S>> Add for Lexicographic<P, S> {
    type Output = Self;

    /// Adds component-wise.
    fn add(self, other: Self) -> Self {
        Self::new(
            self.primary + other.primary,
            self.secondary + other.secondary,
        )
    }
}

impl<P: Cost, S: Cost> Cost for Lexicographic<P, S> {
    const ZERO: Self = Self::new(P::ZERO, S::ZERO);

    /// Adds component-wise, each component saturating on its own.
    fn saturating_add(self, other: Self) -> Self {
        Self::new(
            self.primary.saturating_add(other.primary),
            self.secondary.saturating_add(other.secondary),
        )
    }

    /// Scales component-wise, each component saturating on its own.
    fn saturating_times(self, factor: u32) -> Self {
        Self::new(
            self.primary.saturating_times(factor),
            self.secondary.saturating_times(factor),
        )
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for [`Lexicographic`].

    use proptest::prelude::*;

    use super::{Cost, Lexicographic};
    use crate::Decimal;

    /// Builds a `u32` pair cost.
    fn cost(primary: u32, secondary: u32) -> Lexicographic<u32, u32> {
        Lexicographic::new(primary, secondary)
    }

    /// The primary cost decides; the secondary only breaks ties.
    #[test]
    fn orders_lexicographically() {
        assert!(cost(1, 9) < cost(2, 0));
        assert!(cost(2, 0) < cost(2, 1));
        assert_eq!(cost(3, 3).max(cost(3, 4)), cost(3, 4));
    }

    /// The saturating operations saturate each component on its own.
    #[test]
    fn saturates_per_component() {
        assert_eq!(
            cost(u32::MAX, 1).saturating_add(cost(1, 1)),
            cost(u32::MAX, 2)
        );
        assert_eq!(cost(u32::MAX, 2).saturating_times(2), cost(u32::MAX, 4));
        assert_eq!(cost(5, 7).saturating_times(0), Lexicographic::ZERO);
    }

    /// The components may be different cost types.
    #[test]
    fn mixes_cost_types() {
        let a = Lexicographic::new(Decimal::<2>::from_raw(150), 1_u8);
        let b = Lexicographic::new(Decimal::<2>::from_raw(150), 0_u8);

        assert!(b < a);
        assert_eq!((a + b).primary, Decimal::from_raw(300));
    }

    proptest! {
        /// The order is the order of `(primary, secondary)` tuples.
        #[test]
        fn matches_tuple_order(a: (u32, u32), b: (u32, u32)) {
            prop_assert_eq!(cost(a.0, a.1).cmp(&cost(b.0, b.1)), a.cmp(&b));
        }

        /// A heuristic `(h, 0)` that is consistent for the primary cost stays consistent.
        #[test]
        fn zero_secondary_heuristic_stays_consistent(
            h_u in 0..1_000_u32, c in 0..1_000_u32, h_v in 0..1_000_u32, s: u16,
        ) {
            prop_assume!(h_u <= c + h_v);
            prop_assert!(cost(h_u, 0) <= cost(c, u32::from(s)) + cost(h_v, 0));
        }
    }
}
