//! The checked float cost [`NonNegF64`] (feature `float`).

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Add;

use super::Cost;

/// A non-negative, non-NaN `f64` cost; `+∞` is allowed and acts as the largest cost.
///
/// Raw `f64` cannot implement [`Cost`]: NaN breaks [`Ord`] and negative values break the
/// non-negativity contract. This newtype checks both once, in [`NonNegF64::new`], and keeps
/// the invariant closed under every operation:
///
/// - the sum of two values in `[0, +∞]` is in `[0, +∞]` (a finite overflow becomes `+∞`;
///   `+∞ + +∞ = +∞`), so `+` never produces NaN;
/// - [`Cost::saturating_times`] with factor `0` returns zero instead of computing
///   `+∞ × 0 = NaN`;
/// - `-0.0` is stored as `+0.0`, so [`Eq`], [`Ord`] and [`Hash`] agree (bitwise hashing would
///   otherwise tell the two zeros apart).
///
/// # Rounding
///
/// Float addition rounds and is not associative: the same path summed in a different order
/// can differ in the last bits. Searches stay terminating and panic-free, but "optimal" then
/// means optimal up to rounding, and ties may break differently than with exact arithmetic.
/// Prefer an integer or [`Decimal`](crate::Decimal) cost when exact results matter.
///
/// # Examples
///
/// ```
/// use search_core::{Cost, NonNegF64};
///
/// let a = NonNegF64::new(1.5).unwrap();
/// let b = NonNegF64::new(2.25).unwrap();
///
/// assert_eq!((a + b).get(), 3.75);
/// assert!(NonNegF64::ZERO < a);
/// assert_eq!(NonNegF64::new(f64::NAN), None);
/// assert_eq!(NonNegF64::new(-1.0), None);
/// ```
#[derive(Clone, Copy, Default)]
pub struct NonNegF64(f64);

impl NonNegF64 {
    /// The cost `+∞`: larger than every finite cost.
    pub const INFINITY: Self = Self(f64::INFINITY);

    /// Wraps `value`, or returns `None` if it is NaN or negative.
    ///
    /// `-0.0` is accepted and stored as `+0.0`. `+∞` is accepted.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        if value.is_nan() || value < 0.0 {
            return None;
        }

        // `-0.0 + 0.0` is `+0.0`; every other accepted value is unchanged.
        Some(Self(value + 0.0))
    }

    /// The wrapped value: never NaN, never negative, never `-0.0`.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for NonNegF64 {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for NonNegF64 {}

impl PartialOrd for NonNegF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonNegF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        // Without NaN and `-0.0`, `total_cmp` is the usual numeric order.
        self.0.total_cmp(&other.0)
    }
}

impl Hash for NonNegF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Add for NonNegF64 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Cost for NonNegF64 {
    const ZERO: Self = Self(0.0);

    fn saturating_add(self, other: Self) -> Self {
        self + other
    }

    fn saturating_times(self, factor: u32) -> Self {
        if factor == 0 {
            return Self::ZERO;
        }

        Self(self.0 * f64::from(factor))
    }
}

impl fmt::Display for NonNegF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for NonNegF64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl From<NonNegF64> for f64 {
    fn from(cost: NonNegF64) -> Self {
        cost.0
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for [`NonNegF64`]: the invariant survives every operation.

    use super::{Cost, NonNegF64};
    use proptest::prelude::*;

    /// NaN and negative values are rejected; zeros and infinity are accepted.
    #[test]
    fn new_checks_invariant() {
        assert_eq!(NonNegF64::new(f64::NAN), None);
        assert_eq!(NonNegF64::new(-1e-300), None);
        assert_eq!(NonNegF64::new(f64::NEG_INFINITY), None);
        assert_eq!(NonNegF64::new(f64::INFINITY), Some(NonNegF64::INFINITY));
        assert_eq!(NonNegF64::new(0.0), Some(NonNegF64::ZERO));
    }

    /// `-0.0` is normalized, so it equals, orders and hashes like `+0.0`.
    #[test]
    fn negative_zero_is_normalized() {
        let negative = NonNegF64::new(-0.0).unwrap();
        assert!(negative.get().is_sign_positive());
        assert_eq!(negative, NonNegF64::ZERO);
        assert_eq!(negative.get().to_bits(), 0.0_f64.to_bits());
    }

    /// Infinity times zero is zero, not NaN.
    #[test]
    fn infinity_times_zero_is_zero() {
        assert_eq!(NonNegF64::INFINITY.saturating_times(0), NonNegF64::ZERO);
        assert_eq!(NonNegF64::INFINITY.saturating_times(3), NonNegF64::INFINITY);
    }

    /// Finite overflow saturates at infinity.
    #[test]
    fn overflow_reaches_infinity() {
        let max = NonNegF64::new(f64::MAX).unwrap();
        assert_eq!(max + max, NonNegF64::INFINITY);
        assert_eq!(
            NonNegF64::INFINITY + NonNegF64::INFINITY,
            NonNegF64::INFINITY
        );
    }

    /// Generates any `f64` accepted by [`NonNegF64::new`], including both zeros and `+∞`.
    fn cost() -> impl Strategy<Value = NonNegF64> {
        prop_oneof![
            Just(-0.0),
            Just(f64::INFINITY),
            Just(f64::MAX),
            0.0..f64::MAX,
        ]
        .prop_map(|value| NonNegF64::new(value).expect("generated values are valid"))
    }

    proptest! {
        /// Sums and products stay inside the invariant: not NaN, not negative, not `-0.0`.
        #[test]
        fn operations_preserve_invariant(a in cost(), b in cost(), factor: u32) {
            for value in [a + b, a.saturating_add(b), a.saturating_times(factor)] {
                prop_assert!(!value.get().is_nan());
                prop_assert!(value.get().is_sign_positive());
                prop_assert!(value >= NonNegF64::ZERO);
            }
        }

        /// `Ord` agrees with the numeric order of the wrapped values.
        #[test]
        fn order_matches_numeric(a in cost(), b in cost()) {
            prop_assert_eq!(a.cmp(&b), a.get().partial_cmp(&b.get()).unwrap());
            prop_assert_eq!(a == b, a.get().to_bits() == b.get().to_bits());
        }
    }
}
