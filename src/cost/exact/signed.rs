//! The signed exact integer [`BigInt`], for differences of exact cost
//! expressions that may go negative.

use core::cmp::Ordering;
use core::ops::{Add, Mul, Neg, Sub};

use super::{BigUint, ExactNumber};

/// An arbitrary-precision signed integer: a sign and a [`BigUint`] magnitude.
///
/// Costs are never negative, but expressions over them can be: a parametric weight
/// `den · c₁ - num · c₂` (Dinkelbach's method for ratio objectives), a reduced cost, or the
/// difference of two path sums. `BigInt` holds such values exactly; `+`, `-` and `*` never
/// overflow or round.
///
/// Zero has a single representation, so it is never negative and `-0 == 0`.
///
/// # Examples
///
/// ```
/// use search_core::{BigInt, ExactArith};
///
/// // den · c₁ - num · c₂ for one edge, with 128-bit ratio terms and 64-bit costs.
/// let weight = |den: u128, c1: u64, num: u128, c2: u64| {
///     BigInt::from(den.exact() * c1.exact()) - BigInt::from(num.exact() * c2.exact())
/// };
///
/// let w = weight(u128::MAX, 2, u128::MAX, 3);
/// assert!(w.is_negative());
/// assert_eq!(w + weight(u128::MAX, 1, 0, 0), BigInt::from(0_u128));
/// assert!(BigInt::from(-5_i128) < BigInt::from(3_i128));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct BigInt {
    /// `true` for values below zero. Never `true` for zero.
    negative: bool,
    /// The absolute value.
    magnitude: BigUint,
}

impl BigInt {
    /// Builds `±magnitude`, keeping zero non-negative.
    fn from_parts(negative: bool, magnitude: BigUint) -> Self {
        let negative = negative && !magnitude.limbs().is_empty();

        Self {
            negative,
            magnitude,
        }
    }

    /// `true` if the value is below zero.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// The absolute value.
    #[must_use]
    pub fn magnitude(&self) -> &BigUint {
        &self.magnitude
    }
}

impl From<BigUint> for BigInt {
    fn from(magnitude: BigUint) -> Self {
        Self::from_parts(false, magnitude)
    }
}

impl From<u128> for BigInt {
    fn from(value: u128) -> Self {
        Self::from(BigUint::from(value))
    }
}

impl From<i128> for BigInt {
    fn from(value: i128) -> Self {
        Self::from_parts(value < 0, BigUint::from(value.unsigned_abs()))
    }
}

impl Neg for BigInt {
    type Output = Self;

    fn neg(self) -> Self {
        Self::from_parts(!self.negative, self.magnitude)
    }
}

impl Add for BigInt {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.negative == other.negative {
            return Self::from_parts(self.negative, self.magnitude + other.magnitude);
        }

        // Opposite signs: the larger magnitude keeps its sign.
        if self.magnitude >= other.magnitude {
            Self::from_parts(self.negative, self.magnitude.minus(&other.magnitude))
        } else {
            Self::from_parts(other.negative, other.magnitude.minus(&self.magnitude))
        }
    }
}

impl Sub for BigInt {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self + -other
    }
}

impl Mul for BigInt {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self::from_parts(
            self.negative != other.negative,
            self.magnitude * other.magnitude,
        )
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigInt {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => self.magnitude.cmp(&other.magnitude),
            (true, true) => other.magnitude.cmp(&self.magnitude),
        }
    }
}

impl ExactNumber for BigInt {
    fn zero() -> Self {
        Self::from(BigUint::zero())
    }

    fn one() -> Self {
        Self::from(BigUint::one())
    }

    fn abs_diff(&self, other: &Self) -> Self {
        let difference = self.clone() - other.clone();

        Self::from(difference.magnitude)
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for [`BigInt`], against `i128` arithmetic.

    use proptest::prelude::*;

    use super::{BigInt, BigUint, ExactNumber};

    /// The value of a big integer as `i128`, for values that fit.
    fn value(b: &BigInt) -> i128 {
        let magnitude = b
            .magnitude()
            .limbs()
            .iter()
            .rev()
            .fold(0_i128, |v, &l| (v << 32) | i128::from(l));

        if b.is_negative() {
            -magnitude
        } else {
            magnitude
        }
    }

    /// Zero is never negative, however it is reached.
    #[test]
    fn zero_is_not_negative() {
        let five = BigInt::from(5_i128);

        assert!(!(five.clone() - five.clone()).is_negative());
        assert!(!(-BigInt::zero()).is_negative());
        assert!(!(BigInt::from(-5_i128) * BigInt::zero()).is_negative());
        assert_eq!(-BigInt::zero(), BigInt::zero());
    }

    /// Values beyond `i128` stay exact.
    #[test]
    fn exceeds_i128() {
        let max = BigInt::from(i128::MAX);
        let min = BigInt::from(i128::MIN);

        assert_eq!(max.clone() - min.clone() - max.clone(), -min.clone());
        assert!(min.clone() * max.clone() < min);
        assert_eq!(
            BigInt::from(i128::MIN).magnitude(),
            &BigUint::from(1_u128 << 127)
        );
    }

    proptest! {
        /// `+`, `-` and `*` agree with `i128` on 64-bit inputs.
        #[test]
        fn matches_i128(a: i64, b: i64) {
            let (x, y) = (BigInt::from(i128::from(a)), BigInt::from(i128::from(b)));
            let (a, b) = (i128::from(a), i128::from(b));

            prop_assert_eq!(value(&(x.clone() + y.clone())), a + b);
            prop_assert_eq!(value(&(x.clone() - y.clone())), a - b);
            prop_assert_eq!(value(&(x.clone() * y.clone())), a * b);
            prop_assert_eq!(value(&x.abs_diff(&y)), (a - b).abs());
            prop_assert_eq!(value(&-x.clone()), -a);
        }

        /// The order agrees with `i128`.
        #[test]
        fn order_matches_i128(a: i128, b: i128) {
            prop_assert_eq!(BigInt::from(a).cmp(&BigInt::from(b)), a.cmp(&b));
        }
    }
}
