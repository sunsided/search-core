//! Priority arithmetic on costs: [`CostExt`] adds saturating subtraction and a
//! conversion to `f64`, which [`Cost`] deliberately lacks.

#[cfg(feature = "float")]
use super::NonNegF64;
use super::{Cost, Decimal};

/// Priority arithmetic that [`Cost`] lacks: subtraction and conversion to `f64`.
///
/// Some searches order nodes by priorities that subtract or divide costs: the `(G - g) / h`
/// order of anytime nonparametric A*, the `2g + h - h_opposite` key of front-to-end
/// bidirectional searches, or a potential such as dynamic potential search. Those searches
/// require this trait; every other search needs [`Cost`] alone. Like [`WideMul`] and
/// [`ExactArith`], it is a separate trait, so cost types without it keep working.
///
/// | Cost | `saturating_sub` | `to_f64` |
/// |---|---|---|
/// | `u8` to `u128`, `usize` | saturates at `0` | nearest float; exact up to `2^53` |
/// | [`Decimal<K>`] | on raw units, saturates at `0` | the raw unit count, **not** divided by `10^K` |
/// | `NonNegF64` (feature `float`) | clamps at `0`, also `∞ - ∞` | the value itself |
///
/// A user-defined cost type implements it next to [`Cost`].
///
/// [`WideMul`]: crate::WideMul
/// [`ExactArith`]: crate::ExactArith
///
/// # Examples
///
/// ```
/// use search_core::{CostExt, Decimal};
///
/// assert_eq!(CostExt::saturating_sub(3_u32, 5), 0);
/// assert_eq!(7_u64.to_f64(), 7.0);
///
/// // Decimal converts its raw units: 1.50 at scale 10^2 is 150.
/// assert_eq!(Decimal::<2>::from_raw(150).to_f64(), 150.0);
/// ```
pub trait CostExt: Cost {
    /// Subtracts `other`, saturating at [`Cost::ZERO`].
    ///
    /// For priorities that subtract one estimate from another, such as a `G - g` slack.
    #[must_use]
    fn saturating_sub(self, other: Self) -> Self;

    /// The cost as a float, possibly rounded; for [`Decimal<K>`] the raw units, scaled by
    /// `10^K`.
    ///
    /// Only for priorities that divide costs, such as a `(G - g) / h` order. Rounding can
    /// change the expansion order among nearly equal priorities, but never a returned cost.
    /// Within one cost type the result is proportional to the cost, so quotients and the
    /// order of differences are what the exact values give, up to rounding. Do not compare
    /// the result against a threshold in cost units: for [`Decimal<K>`] it is off by `10^K`.
    #[must_use]
    fn to_f64(self) -> f64;
}

/// Implements [`CostExt`] for unsigned integer types, each with its conversion to `f64`.
macro_rules! integer_cost_ext {
    ($($ty:ty => $to_f64:path),*) => {
        $(
            impl CostExt for $ty {
                fn saturating_sub(self, other: Self) -> Self {
                    <$ty>::saturating_sub(self, other)
                }

                fn to_f64(self) -> f64 {
                    $to_f64(self)
                }
            }
        )*
    };
}

integer_cost_ext!(
    u8 => f64::from,
    u16 => f64::from,
    u32 => f64::from,
    u64 => u64_to_f64,
    u128 => u128_to_f64,
    usize => usize_to_f64
);

/// Converts a `u64` cost to the nearest float, for priorities only.
#[expect(clippy::cast_precision_loss, reason = "only used for priorities")]
fn u64_to_f64(cost: u64) -> f64 {
    cost as f64
}

/// Converts a `u128` cost to the nearest float, for priorities only.
#[expect(clippy::cast_precision_loss, reason = "only used for priorities")]
fn u128_to_f64(cost: u128) -> f64 {
    cost as f64
}

/// Converts a `usize` cost to the nearest float, for priorities only.
#[expect(clippy::cast_precision_loss, reason = "only used for priorities")]
fn usize_to_f64(cost: usize) -> f64 {
    cost as f64
}

impl<const K: u32> CostExt for Decimal<K> {
    fn saturating_sub(self, other: Self) -> Self {
        Self::from_raw(self.raw().saturating_sub(other.raw()))
    }

    /// The raw unit count as a float: the value scaled by `10^K`.
    ///
    /// Priorities only compare quotients and differences of costs of one type, so the common
    /// factor `10^K` cancels, and dividing it out would only add rounding.
    fn to_f64(self) -> f64 {
        u64_to_f64(self.raw())
    }
}

#[cfg(feature = "float")]
impl CostExt for NonNegF64 {
    /// Subtracts `other`, clamping at zero; `∞ - ∞` is zero.
    fn saturating_sub(self, other: Self) -> Self {
        // `f64::max` returns `0.0` for a NaN difference, which only `∞ - ∞` produces.
        Self::new((self.get() - other.get()).max(0.0)).unwrap_or(Self::ZERO)
    }

    fn to_f64(self) -> f64 {
        self.get()
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for the [`CostExt`] implementations.

    use proptest::prelude::*;

    use super::{CostExt, Decimal};

    /// Integer subtraction saturates at zero and converts exactly for small values.
    #[test]
    fn integers_saturate_and_convert() {
        assert_eq!(CostExt::saturating_sub(2_u8, 5), 0);
        assert_eq!(CostExt::saturating_sub(9_u128, 4), 5);
        assert_eq!(CostExt::saturating_sub(7_usize, 7), 0);
        assert!((CostExt::to_f64(3_u16) - 3.0).abs() < f64::EPSILON);
        assert!((CostExt::to_f64(u64::from(u32::MAX)) - f64::from(u32::MAX)).abs() < 1.0);
    }

    /// Decimals subtract on raw units and convert to the raw unit count.
    #[test]
    fn decimals_use_raw_units() {
        let a = Decimal::<2>::from_raw(150);
        let b = Decimal::<2>::from_raw(25);

        assert_eq!(a.saturating_sub(b), Decimal::from_raw(125));
        assert_eq!(b.saturating_sub(a), Decimal::from_raw(0));
        assert!((a.to_f64() - 150.0).abs() < f64::EPSILON);
    }

    /// Float subtraction clamps at zero, also for `∞ - ∞`.
    #[cfg(feature = "float")]
    #[test]
    fn floats_clamp_at_zero() {
        use crate::{Cost, NonNegF64};

        let half = NonNegF64::new(0.5).expect("non-negative");
        let two = NonNegF64::new(2.0).expect("non-negative");

        assert_eq!(
            two.saturating_sub(half),
            NonNegF64::new(1.5).expect("non-negative")
        );
        assert_eq!(half.saturating_sub(two), NonNegF64::ZERO);
        assert_eq!(
            NonNegF64::INFINITY.saturating_sub(NonNegF64::INFINITY),
            NonNegF64::ZERO
        );
        assert_eq!(NonNegF64::INFINITY.saturating_sub(two), NonNegF64::INFINITY);
        assert!((two.to_f64() - 2.0).abs() < f64::EPSILON);
    }

    proptest! {
        /// Subtraction undoes addition and never goes below zero.
        #[test]
        fn subtraction_inverts_addition(a in 0..u64::MAX / 2, b in 0..u64::MAX / 2) {
            prop_assert_eq!(CostExt::saturating_sub(a + b, b), a);
            prop_assert_eq!(CostExt::saturating_sub(b, a + b + 1), 0);
        }

        /// The float conversion preserves the order of costs.
        #[test]
        fn conversion_is_monotone(a: u64, b: u64) {
            if a <= b {
                prop_assert!(a.to_f64() <= b.to_f64());
            }
        }
    }
}
