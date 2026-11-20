//! The exact cost-by-cost product [`WideMul`] and its implementations.

use super::Decimal;
#[cfg(feature = "float")]
use super::NonNegF64;

/// The product of two costs, in a type wide enough to hold it.
///
/// [`Cost`](crate::Cost) can only scale a cost by a small integer. Some searches need to
/// compare ratios of costs, `a / b` against `c / d`, which they do without division by
/// comparing the cross products `a · d` and `c · b`. This trait gives those products exactly:
/// for the integer types and [`Decimal`] the product never overflows and never rounds, so
/// `a.wide_mul(d) < c.wide_mul(b)` holds exactly when `a · d < c · b`.
///
/// The ratios such searches compare are often differences over costs, as in
/// `(x - y) / z`, and [`Cost`](crate::Cost) has no subtraction. [`WideMul::abs_diff`] gives
/// the exact difference, which never leaves the type's range.
///
/// It is a separate trait rather than a part of [`Cost`](crate::Cost), so that cost types
/// without a product keep working with every algorithm that does not need one.
///
/// | Cost | `Wide` | Exact |
/// |---|---|---|
/// | `u8`, `u16`, `u32`, `u64` | the next wider unsigned integer | yes |
/// | `usize` | `u128` | yes |
/// | `u128` | `(u128, u128)`, the high and low halves of the 256-bit product | yes |
/// | [`Decimal<K>`] | `u128`, the product of the raw values (scale `10^(2K)`) | yes |
/// | `NonNegF64` (feature `float`) | `NonNegF64` | rounded like `f64` multiplication |
///
/// # Examples
///
/// Comparing `3 / 4` with `5 / 7` in `u64` without division or overflow:
///
/// ```
/// use search_core::WideMul;
///
/// /// Returns `true` if `a / b < c / d`, for positive `b` and `d`.
/// fn ratio_less<T: WideMul>(a: T, b: T, c: T, d: T) -> bool {
///     a.wide_mul(d) < c.wide_mul(b)
/// }
///
/// assert!(!ratio_less(3_u64, 4, 5, 7));
/// assert!(ratio_less(u64::MAX - 1, u64::MAX, 1, 1));
/// ```
pub trait WideMul: Copy {
    /// The type of the product, ordered like the numbers it holds.
    type Wide: Copy + Ord;

    /// Multiplies two costs into [`WideMul::Wide`].
    #[must_use]
    fn wide_mul(self, other: Self) -> Self::Wide;

    /// The distance `|self - other|`, exact for the integers and [`Decimal`].
    #[must_use]
    fn abs_diff(self, other: Self) -> Self;
}

/// Implements [`WideMul`] for unsigned integers that widen losslessly into `$wide`.
macro_rules! widening {
    ($($ty:ty => $wide:ty),*) => {
        $(
            impl WideMul for $ty {
                type Wide = $wide;

                fn wide_mul(self, other: Self) -> $wide {
                    <$wide>::from(self) * <$wide>::from(other)
                }

                fn abs_diff(self, other: Self) -> Self {
                    <$ty>::abs_diff(self, other)
                }
            }
        )*
    };
}

widening!(u8 => u16, u16 => u32, u32 => u64, u64 => u128);

impl WideMul for usize {
    type Wide = u128;

    fn wide_mul(self, other: Self) -> u128 {
        // `usize` is at most 64 bits on every target Rust supports, so this never fails.
        let widen = |x: usize| u128::from(u64::try_from(x).expect("usize fits in u64"));
        widen(self) * widen(other)
    }

    fn abs_diff(self, other: Self) -> Self {
        usize::abs_diff(self, other)
    }
}

impl WideMul for u128 {
    /// `(high, low)`: the product is `high · 2^128 + low`. Tuples compare lexicographically,
    /// which is the numeric order here.
    type Wide = (u128, u128);

    fn wide_mul(self, other: Self) -> (u128, u128) {
        mul_u128(self, other)
    }

    fn abs_diff(self, other: Self) -> Self {
        u128::abs_diff(self, other)
    }
}

/// The full 256-bit product of `a` and `b` as `(high, low)` 128-bit halves.
///
/// Schoolbook multiplication on 64-bit limbs: each limb product fits in `u128`, and the
/// middle sum is split so that no intermediate addition overflows.
fn mul_u128(a: u128, b: u128) -> (u128, u128) {
    const LOW: u128 = u64::MAX as u128;

    let (a_hi, a_lo) = (a >> 64, a & LOW);
    let (b_hi, b_lo) = (b >> 64, b & LOW);

    let lo_lo = a_lo * b_lo;
    let hi_lo = a_hi * b_lo;
    let lo_hi = a_lo * b_hi;
    let hi_hi = a_hi * b_hi;

    // Bits 64..192, as three numbers below 2^64 each, summed: below 3 · 2^64.
    let middle = (lo_lo >> 64) + (hi_lo & LOW) + (lo_hi & LOW);
    let low = (middle << 64) | (lo_lo & LOW);
    let high = hi_hi + (hi_lo >> 64) + (lo_hi >> 64) + (middle >> 64);

    (high, low)
}

impl<const K: u32> WideMul for Decimal<K> {
    /// The product of the raw values, at scale `10^(2K)`.
    type Wide = u128;

    fn wide_mul(self, other: Self) -> u128 {
        self.raw().wide_mul(other.raw())
    }

    fn abs_diff(self, other: Self) -> Self {
        Self::from_raw(self.raw().abs_diff(other.raw()))
    }
}

#[cfg(feature = "float")]
impl WideMul for NonNegF64 {
    type Wide = Self;

    /// The `f64` product, with `0 · ∞ = 0` instead of NaN.
    fn wide_mul(self, other: Self) -> Self {
        if self.get() == 0.0 || other.get() == 0.0 {
            return Self::new(0.0).expect("zero is a valid cost");
        }

        Self::new(self.get() * other.get()).expect("a product of non-negative non-NaN values")
    }

    /// The `f64` distance, with `∞ - ∞ = 0` instead of NaN.
    fn abs_diff(self, other: Self) -> Self {
        if self == other {
            return Self::new(0.0).expect("zero is a valid cost");
        }

        Self::new((self.get() - other.get()).abs()).expect("a distance of unequal values")
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for [`WideMul`].

    use super::{Decimal, WideMul, mul_u128};
    use proptest::prelude::*;

    /// The largest products of each type are exact.
    #[test]
    fn extremes_are_exact() {
        assert_eq!(u8::MAX.wide_mul(u8::MAX), 65_025);
        assert_eq!(
            u64::MAX.wide_mul(u64::MAX),
            u128::from(u64::MAX) * u128::from(u64::MAX)
        );
        assert_eq!(u128::MAX.wide_mul(u128::MAX), (u128::MAX - 1, 1));
        assert_eq!(u128::MAX.wide_mul(0), (0, 0));
        assert_eq!(usize::MAX.wide_mul(2), 2 * u128::from(u64::MAX));
    }

    /// Distances are symmetric and exact at the extremes.
    #[test]
    fn abs_diff_is_exact() {
        assert_eq!(WideMul::abs_diff(3_u8, 250), 247);
        assert_eq!(WideMul::abs_diff(u128::MAX, 0), u128::MAX);
        let (a, b) = (Decimal::<1>::from_raw(5), Decimal::<1>::from_raw(12));
        assert_eq!(WideMul::abs_diff(a, b), Decimal::from_raw(7));
    }

    /// [`Decimal`] multiplies its raw values.
    #[test]
    fn decimal_multiplies_raw_values() {
        let a = Decimal::<2>::from_raw(150);
        let b = Decimal::<2>::from_raw(u64::MAX);
        assert_eq!(a.wide_mul(b), 150 * u128::from(u64::MAX));
    }

    /// Zero times infinity is zero, and products round like `f64`.
    #[cfg(feature = "float")]
    #[test]
    fn float_products() {
        use crate::NonNegF64;

        let inf = NonNegF64::new(f64::INFINITY).unwrap();
        let zero = NonNegF64::new(0.0).unwrap();
        let half = NonNegF64::new(0.5).unwrap();
        assert_eq!(inf.wide_mul(zero), zero);
        assert_eq!(half.wide_mul(half).get(), 0.25);
        assert_eq!(inf.wide_mul(half), inf);
        assert_eq!(WideMul::abs_diff(inf, inf), zero);
        assert_eq!(WideMul::abs_diff(half, inf), inf);
    }

    proptest! {
        /// The 256-bit product splits back into the 128-bit operands' product when it fits,
        /// and agrees with a product of the 64-bit halves computed independently.
        #[test]
        fn u128_matches_limb_reference(a: u128, b: u128) {
            let (high, low) = mul_u128(a, b);
            prop_assert_eq!(low, a.wrapping_mul(b));
            if let Some(exact) = a.checked_mul(b) {
                prop_assert_eq!((high, low), (0, exact));
            }

            // a · b = (a / k) · b · k + (a mod k) · b, with k = 2: compare via the shifted
            // product, which halves `a` and so cannot be the same computation.
            let (h2, l2) = mul_u128(a >> 1, b);
            let doubled = ((h2 << 1) | (l2 >> 127), l2 << 1);
            let odd = if a & 1 == 1 { b } else { 0 };
            let (sum_low, carry) = doubled.1.overflowing_add(odd);
            prop_assert_eq!((high, low), (doubled.0 + u128::from(carry), sum_low));
        }

        /// The widening products agree with the independent 256-bit limb product.
        #[test]
        fn widening_matches_limb_product(a: u64, b: u64, c: u32, d: u32, e: u16, f: u16) {
            prop_assert_eq!(mul_u128(a.into(), b.into()), (0, a.wide_mul(b)));
            prop_assert_eq!(mul_u128(c.into(), d.into()), (0, c.wide_mul(d).into()));
            prop_assert_eq!(mul_u128(e.into(), f.into()), (0, e.wide_mul(f).into()));
        }
    }
}
