//! The fixed-point decimal cost [`Decimal`].

use core::fmt;
use core::ops::Add;

use super::Cost;

/// A non-negative fixed-point decimal cost with `K` decimal places.
///
/// Stores an integer number of units of `10^-K` in a `u64`, so `Decimal::<2>` counts
/// hundredths. Arithmetic is exact integer arithmetic: sums never round and are independent
/// of the summation order, which floats cannot promise. The scale is part of the type, so
/// costs of different scales cannot be mixed by accident.
///
/// `K` ranges from `0` to `19`; a larger `K` is rejected at compile time as soon as a
/// constructor, [`Decimal::SCALE`] or formatting is used. The largest representable cost is
/// `u64::MAX / 10^K` whole units plus the fraction; `+` overflows like `u64` (see the
/// [`Cost`] contract).
///
/// ```compile_fail
/// // 10^20 does not fit into u64.
/// let _ = search_core::Decimal::<20>::from_raw(1);
/// ```
///
/// [`Default`] is zero and, being derived, skips that check; it is harmless because every
/// operation that needs the scale performs it.
///
/// [`Display`](fmt::Display) and [`Debug`](fmt::Debug) both print the decimal value with
/// exactly `K` fraction digits.
///
/// # Examples
///
/// ```
/// use search_core::{Cost, Decimal};
///
/// type Cents = Decimal<2>;
///
/// let step = Cents::from_raw(125); // 1.25
/// let bonus = Cents::from_int(2).unwrap();
///
/// assert_eq!(step + bonus, Cents::from_raw(325));
/// assert_eq!((step + bonus).to_string(), "3.25");
/// assert!(Cents::ZERO < step);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Decimal<const K: u32>(u64);

impl<const K: u32> Decimal<K> {
    /// The number of raw units in one whole unit: `10^K`.
    ///
    /// Referencing it with `K > 19` is a compile-time error, because `10^20` exceeds `u64`.
    pub const SCALE: u64 = {
        assert!(K <= 19, "Decimal supports at most 19 decimal places");
        10_u64.pow(K)
    };

    /// Creates a cost from its raw number of `10^-K` units.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        let _ = Self::SCALE;
        Self(raw)
    }

    /// Creates the cost of `whole` full units, or `None` if `whole * 10^K` overflows `u64`.
    #[must_use]
    pub const fn from_int(whole: u64) -> Option<Self> {
        match whole.checked_mul(Self::SCALE) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }

    /// The raw number of `10^-K` units.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

impl<const K: u32> Add for Decimal<K> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl<const K: u32> Cost for Decimal<K> {
    const ZERO: Self = Self::from_raw(0);

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    fn saturating_times(self, factor: u32) -> Self {
        Self(self.0.saturating_mul(u64::from(factor)))
    }
}

impl<const K: u32> fmt::Display for Decimal<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let whole = self.0 / Self::SCALE;
        if K == 0 {
            return write!(f, "{whole}");
        }

        let fraction = self.0 % Self::SCALE;
        let digits = K as usize;

        write!(f, "{whole}.{fraction:0digits$}")
    }
}

impl<const K: u32> fmt::Debug for Decimal<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for [`Decimal`].

    extern crate std;

    use std::string::ToString;

    use super::{Cost, Decimal};
    use proptest::prelude::*;

    /// Formatting pads the fraction to exactly `K` digits.
    #[test]
    fn display_pads_fraction() {
        assert_eq!(Decimal::<2>::from_raw(5).to_string(), "0.05");
        assert_eq!(Decimal::<3>::from_raw(12_000).to_string(), "12.000");
        assert_eq!(format_debug(Decimal::<1>::from_raw(15)), "1.5");
    }

    /// `K = 0` prints a plain integer without a decimal point.
    #[test]
    fn display_without_fraction() {
        assert_eq!(Decimal::<0>::from_raw(42).to_string(), "42");
        assert_eq!(Decimal::<0>::SCALE, 1);
    }

    /// The largest scale prints all 19 fraction digits of `u64::MAX`.
    #[test]
    fn display_largest_scale() {
        assert_eq!(
            Decimal::<19>::from_raw(u64::MAX).to_string(),
            "1.8446744073709551615"
        );
        assert_eq!(Decimal::<19>::SCALE, 10_000_000_000_000_000_000);
    }

    /// `from_int` multiplies by the scale and rejects overflow.
    #[test]
    fn from_int_checks_overflow() {
        assert_eq!(Decimal::<3>::from_int(7).map(Decimal::raw), Some(7_000));
        assert_eq!(
            Decimal::<19>::from_int(1).map(Decimal::raw),
            Some(Decimal::<19>::SCALE)
        );
        assert_eq!(Decimal::<19>::from_int(2), None);
    }

    /// The saturating operations clamp at `u64::MAX` raw units.
    #[test]
    fn saturating_operations_clamp() {
        let max = Decimal::<2>::from_raw(u64::MAX);
        assert_eq!(max.saturating_add(Decimal::from_raw(1)), max);
        assert_eq!(max.saturating_times(2), max);
        assert_eq!(max.saturating_times(0), Decimal::ZERO);
    }

    /// Formats a value with [`Debug`](core::fmt::Debug).
    fn format_debug(value: Decimal<1>) -> std::string::String {
        std::format!("{value:?}")
    }

    proptest! {
        /// Order and addition agree with the raw `u64` values.
        #[test]
        fn matches_raw_arithmetic(a in 0..u64::MAX / 2, b in 0..u64::MAX / 2) {
            let (x, y) = (Decimal::<4>::from_raw(a), Decimal::<4>::from_raw(b));
            prop_assert_eq!((x + y).raw(), a + b);
            prop_assert_eq!(x.cmp(&y), a.cmp(&b));
        }

        /// Formatting round-trips: whole part and padded fraction reproduce the raw value.
        #[test]
        fn display_round_trips(raw: u64) {
            let text = Decimal::<6>::from_raw(raw).to_string();
            let (whole, fraction) = text.split_once('.').expect("K > 0 prints a point");
            prop_assert_eq!(fraction.len(), 6);
            let parsed = whole.parse::<u64>().unwrap() * 1_000_000 + fraction.parse::<u64>().unwrap();
            prop_assert_eq!(parsed, raw);
        }
    }
}
