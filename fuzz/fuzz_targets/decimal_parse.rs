//! Fuzzes [`Decimal`] parsing against an independent `u128` oracle: arbitrary
//! text either matches the oracle's exact value or is rejected by both.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use search_core::Decimal;

/// The decimal places of the fuzzed type.
const K: u32 = 4;

/// The exact raw value of `whole[.fraction]` at scale `10^K`, or `None` if the
/// text is malformed, has more than `K` fraction digits or exceeds `u64`.
fn oracle(text: &str) -> Option<u64> {
    let (whole, fraction) = text
        .split_once('.')
        .map_or((text, None), |(w, f)| (w, Some(f)));
    let is_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());

    if !is_digits(whole) || fraction.is_some_and(|f| !is_digits(f) || f.len() > K as usize) {
        return None;
    }

    let mut padded = String::from(fraction.unwrap_or(""));
    while padded.len() < K as usize {
        padded.push('0');
    }

    // Leading zeros can make the whole part arbitrarily long without changing its value.
    let whole = whole.trim_start_matches('0');
    if whole.len() > 25 {
        return None;
    }

    let whole: u128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let raw = whole * 10_u128.pow(K) + padded.parse::<u128>().ok()?;

    u64::try_from(raw).ok()
}

fuzz_target!(|text: &str| {
    let parsed = text.parse::<Decimal<K>>().ok().map(Decimal::raw);

    assert_eq!(parsed, oracle(text), "parsing {text:?}");

    if let Some(raw) = parsed {
        let cost = Decimal::<K>::from_raw(raw);
        assert_eq!(
            cost.to_string().parse::<Decimal<K>>(),
            Ok(cost),
            "round trip of {text:?}"
        );
    }
});
