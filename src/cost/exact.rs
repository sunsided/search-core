//! Exact arithmetic beyond one product: [`ExactArith`] maps a cost into an
//! [`ExactNumber`] type where sums and products of any size are exact.

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::ops::{Add, Mul};

#[cfg(feature = "float")]
use super::NonNegF64;
use super::{Cost, Decimal};

mod signed;

pub use signed::BigInt;

/// A number type closed under exact `+` and `*`, with a zero, a one and a
/// distance, for comparisons that [`WideMul`](crate::WideMul) cannot
/// express: sums of products, products of three or more costs.
///
/// Implemented by [`BigUint`] and [`BigInt`] (exact) and, behind `float`, by
/// `NonNegF64` (rounded like `f64`).
pub trait ExactNumber: Clone + Ord + Add<Output = Self> + Mul<Output = Self> {
    /// Zero.
    fn zero() -> Self;

    /// One.
    fn one() -> Self;

    /// `|self - other|`.
    #[must_use]
    fn abs_diff(&self, other: &Self) -> Self;
}

/// A cost that converts into an [`ExactNumber`] without loss.
///
/// An additive companion of [`Cost`] and [`WideMul`](crate::WideMul): cost
/// types without it keep working everywhere else.
///
/// | Cost | `Exact` |
/// |---|---|
/// | `u8` to `u128`, `usize` | [`BigUint`] |
/// | [`Decimal<K>`] | [`BigUint`] of the raw value (scale `10^K`) |
/// | `NonNegF64` (feature `float`) | `NonNegF64` |
///
/// # Examples
///
/// ```
/// use search_core::ExactArith;
///
/// // (d1 · c1 + s) · c2 against (d2 · c2 + s) · c1 without overflow.
/// let e = |x: u64| x.exact();
/// let left = (e(u64::MAX) * e(u64::MAX) + e(7)) * e(u64::MAX);
/// let right = (e(1) * e(u64::MAX) + e(7)) * e(u64::MAX);
/// assert!(left > right);
/// ```
pub trait ExactArith: Cost {
    /// The exact number type.
    type Exact: ExactNumber;

    /// The cost as an exact number.
    #[must_use]
    fn exact(self) -> Self::Exact;
}

/// An arbitrary-precision unsigned integer: 32-bit limbs, least
/// significant first, without most significant zero limbs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct BigUint(Vec<u32>);

impl BigUint {
    /// The limbs, least significant first; empty for zero.
    #[must_use]
    pub fn limbs(&self) -> &[u32] {
        &self.0
    }

    /// Drops the most significant zero limbs.
    fn trimmed(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }

        self
    }

    /// `self - other` for `self >= other`.
    fn minus(&self, other: &Self) -> Self {
        let mut out = self.0.clone();
        let mut borrow = 0_i64;
        for (i, slot) in out.iter_mut().enumerate() {
            let sub = i64::from(other.0.get(i).copied().unwrap_or(0)) + borrow;
            let mut v = i64::from(*slot) - sub;
            borrow = 0;
            if v < 0 {
                v += 1 << 32;
                borrow = 1;
            }

            *slot = u32::try_from(v).expect("a limb after borrowing");
        }

        Self(out).trimmed()
    }
}

impl From<u128> for BigUint {
    fn from(mut value: u128) -> Self {
        let mut limbs = Vec::new();
        while value > 0 {
            limbs.push(u32::try_from(value & u128::from(u32::MAX)).expect("masked"));
            value >>= 32;
        }

        Self(limbs)
    }
}

impl Add for BigUint {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let n = self.0.len().max(other.0.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0_u64;
        for i in 0..n {
            let s = u64::from(self.0.get(i).copied().unwrap_or(0))
                + u64::from(other.0.get(i).copied().unwrap_or(0))
                + carry;
            out.push(u32::try_from(s & u64::from(u32::MAX)).expect("masked"));
            carry = s >> 32;
        }

        out.push(u32::try_from(carry).expect("a carry bit"));
        Self(out).trimmed()
    }
}

impl Mul for BigUint {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let mut out = alloc::vec![0_u32; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0_u64;
            for (j, &b) in other.0.iter().enumerate() {
                let t = u64::from(a) * u64::from(b) + u64::from(out[i + j]) + carry;
                out[i + j] = u32::try_from(t & u64::from(u32::MAX)).expect("masked");
                carry = t >> 32;
            }

            out[i + other.0.len()] = u32::try_from(carry).expect("below 2^32");
        }

        Self(out).trimmed()
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigUint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

impl ExactNumber for BigUint {
    fn zero() -> Self {
        Self(Vec::new())
    }

    fn one() -> Self {
        Self(alloc::vec![1])
    }

    fn abs_diff(&self, other: &Self) -> Self {
        if self >= other {
            self.minus(other)
        } else {
            other.minus(self)
        }
    }
}

/// Implements [`ExactArith`] for unsigned integers that widen into `u128`.
macro_rules! exact_unsigned {
    ($($ty:ty),*) => {
        $(
            impl ExactArith for $ty {
                type Exact = BigUint;

                fn exact(self) -> BigUint {
                    BigUint::from(u128::from(self))
                }
            }
        )*
    };
}

exact_unsigned!(u8, u16, u32, u64, u128);

impl ExactArith for usize {
    type Exact = BigUint;

    fn exact(self) -> BigUint {
        // `usize` is at most 64 bits on every target Rust supports.
        BigUint::from(u128::from(u64::try_from(self).expect("usize fits in u64")))
    }
}

impl<const K: u32> ExactArith for Decimal<K> {
    /// The raw value, at scale `10^K`.
    type Exact = BigUint;

    fn exact(self) -> BigUint {
        BigUint::from(u128::from(self.raw()))
    }
}

#[cfg(feature = "float")]
impl ExactNumber for NonNegF64 {
    fn zero() -> Self {
        Self::new(0.0).expect("zero is a valid cost")
    }

    fn one() -> Self {
        Self::new(1.0).expect("one is a valid cost")
    }

    fn abs_diff(&self, other: &Self) -> Self {
        crate::WideMul::abs_diff(*self, *other)
    }
}

#[cfg(feature = "float")]
impl ExactArith for NonNegF64 {
    /// Itself: `f64` sums and products round.
    type Exact = Self;

    fn exact(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for [`BigUint`].

    use proptest::prelude::*;

    use super::{BigUint, ExactArith, ExactNumber};

    /// The value of a big integer as `u128`, for values that fit.
    fn value(b: &BigUint) -> u128 {
        b.limbs()
            .iter()
            .rev()
            .fold(0, |v, &l| (v << 32) | u128::from(l))
    }

    /// Zero has no limbs and is the identity.
    #[test]
    fn zero() {
        assert_eq!(BigUint::zero().limbs(), &[] as &[u32]);
        assert_eq!(BigUint::zero() + 5_u64.exact(), 5_u64.exact());
        assert_eq!(BigUint::zero() * 5_u64.exact(), BigUint::zero());
        assert_eq!(BigUint::one() * 5_u64.exact(), 5_u64.exact());
    }

    proptest! {
        /// Sums, products and distances match `u128` where it does not
        /// overflow, and order matches.
        #[test]
        fn matches_u128(a: u64, b: u64, c: u32) {
            let (x, y, z) = (a.exact(), b.exact(), c.exact());
            prop_assert_eq!(value(&(x.clone() + y.clone())), u128::from(a) + u128::from(b));
            prop_assert_eq!(value(&(x.clone() * y.clone())), u128::from(a) * u128::from(b));
            prop_assert_eq!(value(&x.abs_diff(&y)), u128::from(a.abs_diff(b)));
            prop_assert_eq!(x.cmp(&y), a.cmp(&b));
            // Beyond 128 bits: (a · b) · c compared with a · (b · c).
            prop_assert_eq!((x.clone() * y.clone()) * z.clone(), x * (y * z));
        }
    }
}
