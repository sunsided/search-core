//! Exact parsing of [`Decimal`] from decimal text, for costs read from files
//! such as PDDL action costs.

use core::fmt;
use core::str::FromStr;

use super::Decimal;

/// Why a string is not a valid [`Decimal<K>`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ParseDecimalError {
    /// The string, or the whole part before the point, is empty.
    Empty,
    /// A character other than an ASCII digit and one `.` between digits, such as a
    /// sign, whitespace, an exponent or a trailing `.`.
    InvalidDigit,
    /// More fraction digits than the `K` decimal places, even if the extra digits are zeros:
    /// the rule is the digit count, not whether the value could be represented.
    TooManyFractionDigits,
    /// The value exceeds the largest `Decimal<K>`, `u64::MAX` raw units.
    Overflow,
}

impl fmt::Display for ParseDecimalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "cannot parse a decimal cost from an empty string",
            Self::InvalidDigit => "invalid character in a decimal cost",
            Self::TooManyFractionDigits => "more fraction digits than the cost's decimal places",
            Self::Overflow => "decimal cost too large",
        })
    }
}

impl core::error::Error for ParseDecimalError {}

/// Parses `whole` or `whole.fraction`, exactly.
///
/// Both parts are plain ASCII digits; leading zeros are fine. The fraction has
/// 1 to `K` digits, trailing zeros included, so nothing is ever rounded: `"1.25"` is a `Decimal<2>` or
/// `Decimal<3>` but not a `Decimal<1>`, and `Decimal<0>` takes integers only.
///
/// # Errors
///
/// - [`ParseDecimalError::Empty`] for `""` or a missing whole part (`".5"`).
/// - [`ParseDecimalError::InvalidDigit`] for signs, whitespace, exponents, a
///   second `.`, or an empty fraction (`"1."`).
/// - [`ParseDecimalError::TooManyFractionDigits`] for more than `K` fraction digits.
/// - [`ParseDecimalError::Overflow`] when the value exceeds `u64::MAX` raw units.
///
/// # Examples
///
/// ```
/// use search_core::{Decimal, ParseDecimalError};
///
/// assert_eq!("3.25".parse::<Decimal<2>>(), Ok(Decimal::from_raw(325)));
/// assert_eq!("3.2".parse::<Decimal<2>>(), Ok(Decimal::from_raw(320)));
/// assert_eq!("7".parse::<Decimal<2>>(), Ok(Decimal::from_raw(700)));
/// assert_eq!("3.25".parse::<Decimal<1>>(), Err(ParseDecimalError::TooManyFractionDigits));
/// assert_eq!("-1".parse::<Decimal<2>>(), Err(ParseDecimalError::InvalidDigit));
/// ```
impl<const K: u32> FromStr for Decimal<K> {
    type Err = ParseDecimalError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (whole, fraction) = match text.split_once('.') {
            Some((whole, fraction)) => (whole, Some(fraction)),
            None => (text, None),
        };

        if whole.is_empty() {
            return Err(ParseDecimalError::Empty);
        }

        let whole_raw = digits_value(whole)?
            .checked_mul(Self::SCALE)
            .ok_or(ParseDecimalError::Overflow)?;
        let fraction_raw = match fraction {
            Some(fraction) => fraction_raw::<K>(fraction)?,
            None => 0,
        };

        whole_raw
            .checked_add(fraction_raw)
            .map(Self::from_raw)
            .ok_or(ParseDecimalError::Overflow)
    }
}

/// The value of a run of ASCII digits, rejecting anything else and `u64` overflow.
///
/// `str::parse::<u64>` is not used because it accepts a leading `+`.
fn digits_value(digits: &str) -> Result<u64, ParseDecimalError> {
    digits.bytes().try_fold(0_u64, |value, byte| {
        if !byte.is_ascii_digit() {
            return Err(ParseDecimalError::InvalidDigit);
        }

        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
            .ok_or(ParseDecimalError::Overflow)
    })
}

/// The fraction digits as raw `10^-K` units: `"5"` is `5 · 10^(K-1)`.
///
/// Malformed text is [`ParseDecimalError::InvalidDigit`] first. The length is checked before
/// the digits are summed, so an overlong fraction is always
/// [`ParseDecimalError::TooManyFractionDigits`], never `Overflow`. After that check there
/// are at most `K <= 19` digits, so the value is below `10^K` and cannot overflow.
fn fraction_raw<const K: u32>(fraction: &str) -> Result<u64, ParseDecimalError> {
    if fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ParseDecimalError::InvalidDigit);
    }

    let missing = usize::try_from(K)
        .ok()
        .and_then(|k| k.checked_sub(fraction.len()))
        .ok_or(ParseDecimalError::TooManyFractionDigits)?;
    let padding = u32::try_from(missing).expect("missing <= K, which is a u32");
    let value = digits_value(fraction)?;

    Ok(value * 10_u64.pow(padding))
}

#[cfg(test)]
mod tests {
    //! Unit and property tests for parsing [`Decimal`].

    extern crate std;

    use std::string::ToString;

    use proptest::prelude::*;

    use super::{Decimal, ParseDecimalError};

    /// Parses `text` as a `Decimal<K>` and returns its raw value.
    fn raw<const K: u32>(text: &str) -> Result<u64, ParseDecimalError> {
        text.parse::<Decimal<K>>().map(Decimal::raw)
    }

    /// Whole numbers, short fractions and leading zeros parse exactly.
    #[test]
    fn parses_valid_text() {
        assert_eq!(raw::<2>("0"), Ok(0));
        assert_eq!(raw::<2>("12"), Ok(1_200));
        assert_eq!(raw::<2>("1.5"), Ok(150));
        assert_eq!(raw::<2>("1.05"), Ok(105));
        assert_eq!(raw::<2>("007.50"), Ok(750));
        assert_eq!(raw::<0>("42"), Ok(42));
    }

    /// Signs, whitespace, exponents, stray points and empty parts are rejected.
    #[test]
    fn rejects_malformed_text() {
        assert_eq!(raw::<2>(""), Err(ParseDecimalError::Empty));
        assert_eq!(raw::<2>(".5"), Err(ParseDecimalError::Empty));
        for text in ["+1", "-1", " 1", "1 ", "1e2", "1.", "1.2.3", "1,5", "x"] {
            assert_eq!(
                raw::<2>(text),
                Err(ParseDecimalError::InvalidDigit),
                "{text:?}"
            );
        }
    }

    /// More than `K` fraction digits are rejected by count, trailing zeros too, and
    /// regardless of how large the digits are.
    #[test]
    fn rejects_too_many_fraction_digits() {
        assert_eq!(
            raw::<2>("0.99999999999999999999"),
            Err(ParseDecimalError::TooManyFractionDigits)
        );
        assert_eq!(
            raw::<1>("1.25"),
            Err(ParseDecimalError::TooManyFractionDigits)
        );
        assert_eq!(
            raw::<1>("1.50"),
            Err(ParseDecimalError::TooManyFractionDigits)
        );
        assert_eq!(
            raw::<0>("1.0"),
            Err(ParseDecimalError::TooManyFractionDigits)
        );
    }

    /// The bounds of `u64` raw units: the maximum parses, one more unit overflows.
    #[test]
    fn checks_overflow() {
        assert_eq!(raw::<19>("1.8446744073709551615"), Ok(u64::MAX));
        assert_eq!(
            raw::<19>("1.8446744073709551616"),
            Err(ParseDecimalError::Overflow)
        );
        assert_eq!(raw::<19>("2"), Err(ParseDecimalError::Overflow));
        assert_eq!(
            raw::<0>("18446744073709551616"),
            Err(ParseDecimalError::Overflow)
        );
        assert_eq!(
            raw::<2>("99999999999999999999999"),
            Err(ParseDecimalError::Overflow)
        );
    }

    /// Each error has a readable message.
    #[test]
    fn errors_display() {
        assert_eq!(
            ParseDecimalError::TooManyFractionDigits.to_string(),
            "more fraction digits than the cost's decimal places"
        );
    }

    proptest! {
        /// Parsing the printed value gives back the same cost, for every raw value.
        #[test]
        fn display_round_trips(raw: u64) {
            let cost = Decimal::<6>::from_raw(raw);
            prop_assert_eq!(cost.to_string().parse::<Decimal<6>>(), Ok(cost));
        }

        /// Short fractions are scaled, not truncated: `w.f` is `w · 10^K + f · 10^(K - len)`.
        #[test]
        fn scales_short_fractions(whole in 0..1_000_000_u64, fraction in 0..1_000_u64) {
            let text = std::format!("{whole}.{fraction:03}");
            prop_assert_eq!(raw::<5>(&text), Ok(whole * 100_000 + fraction * 100));
        }
    }
}
