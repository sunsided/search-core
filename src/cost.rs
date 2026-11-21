//! Scalar path costs: the [`Cost`] trait, its integer implementations, the fixed-point
//! [`Decimal`] and, behind the `float` feature, the checked float `NonNegF64`; and the exact
//! cost-by-cost product [`WideMul`]; the priority arithmetic [`CostExt`]; and, with `alloc`,
//! exact arithmetic of any size through [`ExactArith`].

mod decimal;
#[cfg(feature = "alloc")]
mod exact;
mod ext;
#[cfg(feature = "float")]
mod float;
mod lexicographic;
mod wide;

use core::ops::Add;

pub use decimal::{Decimal, ParseDecimalError};
#[cfg(feature = "alloc")]
pub use exact::{BigUint, ExactArith, ExactNumber};
pub use ext::CostExt;
#[cfg(feature = "float")]
pub use float::NonNegF64;
pub use lexicographic::Lexicographic;
pub use wide::WideMul;

/// A non-negative scalar edge or path cost.
///
/// Searches add costs along a path and compare the sums. The default cost type of this
/// ecosystem is `u32`.
///
/// # Contract
///
/// - **Non-negative.** Every value is `>=` [`Cost::ZERO`], so extending a path never makes it
///   cheaper. This is why only unsigned integers implement the trait: a negative edge cost
///   silently voids the optimality of Dijkstra, A* and their relatives.
/// - **Total order.** [`Ord`] is the cost order; smaller is cheaper.
/// - **`+` follows the underlying integer.** For the integer types and [`Decimal`], an
///   overflowing `+` panics in debug builds and wraps in release builds, exactly like the
///   primitive. Pick a type wide enough for the longest path, or use
///   [`Cost::saturating_add`] where a sum only serves as a priority. `NonNegF64` never
///   overflows; it reaches infinity instead.
///
/// Implemented for `u8`, `u16`, `u32`, `u64`, `u128`, `usize`, [`Decimal`], the pair
/// [`Lexicographic`] and (feature `float`) `NonNegF64`.
///
/// # Examples
///
/// A function generic over the cost type:
///
/// ```
/// use search_core::Cost;
///
/// fn path_cost<C: Cost>(edges: &[C]) -> C {
///     edges.iter().fold(C::ZERO, |sum, &edge| sum + edge)
/// }
///
/// assert_eq!(path_cost(&[3_u32, 4, 5]), 12);
/// assert_eq!(path_cost::<u8>(&[]), 0);
/// ```
pub trait Cost: Copy + Ord + Add<Output = Self> {
    /// The cost of the empty path, and the smallest value of the type.
    const ZERO: Self;

    /// Adds two costs, saturating at the largest value instead of overflowing.
    ///
    /// Used where a sum only serves as a priority, such as a weighted A* key.
    #[must_use]
    fn saturating_add(self, other: Self) -> Self;

    /// Multiplies the cost by a small integer factor, saturating at the largest value.
    ///
    /// Used to compare weighted sums such as `den * g + num * h` without division. A factor
    /// of `0` always yields [`Cost::ZERO`].
    #[must_use]
    fn saturating_times(self, factor: u32) -> Self;
}

/// Implements [`Cost`] for unsigned integer types that `u32` converts into losslessly.
macro_rules! unsigned_cost {
    ($($ty:ty),*) => {
        $(
            impl Cost for $ty {
                const ZERO: Self = 0;

                fn saturating_add(self, other: Self) -> Self {
                    <$ty>::saturating_add(self, other)
                }

                fn saturating_times(self, factor: u32) -> Self {
                    self.saturating_mul(<$ty>::from(factor))
                }
            }
        )*
    };
}

unsigned_cost!(u32, u64, u128);

/// Implements [`Cost`] for unsigned integer types narrower than `u32`, where a factor that
/// does not fit saturates the product unless the cost is zero.
macro_rules! narrow_unsigned_cost {
    ($($ty:ty),*) => {
        $(
            impl Cost for $ty {
                const ZERO: Self = 0;

                fn saturating_add(self, other: Self) -> Self {
                    <$ty>::saturating_add(self, other)
                }

                fn saturating_times(self, factor: u32) -> Self {
                    match <$ty>::try_from(factor) {
                        Ok(factor) => self.saturating_mul(factor),
                        Err(_) if self == 0 => 0,
                        Err(_) => <$ty>::MAX,
                    }
                }
            }
        )*
    };
}

narrow_unsigned_cost!(u8, u16, usize);

#[cfg(test)]
mod tests {
    //! Unit and property tests for the integer [`Cost`] implementations.

    use super::Cost;
    use proptest::prelude::*;

    /// Saturating addition stops at the maximum.
    #[test]
    fn saturating_add_saturates() {
        assert_eq!(Cost::saturating_add(u8::MAX, 1), u8::MAX);
        assert_eq!(Cost::saturating_add(3_u32, 4), 7);
    }

    /// Narrow types saturate on factors they cannot represent, except for a zero cost.
    #[test]
    fn narrow_times_saturates_on_wide_factor() {
        assert_eq!(2_u8.saturating_times(300), u8::MAX);
        assert_eq!(0_u8.saturating_times(300), 0);
        assert_eq!(7_u16.saturating_times(3), 21);
        assert_eq!(0_u16.saturating_times(u32::MAX), 0);
    }

    /// Wide types multiply exactly when the product fits.
    #[test]
    fn wide_times_is_exact() {
        assert_eq!(5_u64.saturating_times(u32::MAX), 5 * u64::from(u32::MAX));
        assert_eq!(u32::MAX.saturating_times(2), u32::MAX);
        assert_eq!(9_u128.saturating_times(0), 0);
    }

    /// `ZERO` is the smallest value.
    #[test]
    fn zero_is_minimum() {
        assert_eq!(<u8 as Cost>::ZERO, u8::MIN);
        assert_eq!(<usize as Cost>::ZERO, usize::MIN);
    }

    proptest! {
        /// `saturating_times` matches the exact product clamped to the type's range.
        #[test]
        fn u16_times_matches_clamped_product(cost: u16, factor: u32) {
            let exact = u64::from(cost) * u64::from(factor);
            let expected = u16::try_from(exact).unwrap_or(u16::MAX);
            prop_assert_eq!(cost.saturating_times(factor), expected);
        }

        /// `saturating_add` matches the exact sum clamped to the type's range.
        #[test]
        fn u32_add_matches_clamped_sum(a: u32, b: u32) {
            let exact = u64::from(a) + u64::from(b);
            let expected = u32::try_from(exact).unwrap_or(u32::MAX);
            prop_assert_eq!(Cost::saturating_add(a, b), expected);
        }
    }
}
