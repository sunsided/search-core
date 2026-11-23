//! The shareable-inputs convention for search-core itself: its types are `Send + Sync`, so a
//! model or heuristic table built from them can serve every thread.

#[cfg(feature = "float")]
use search_core::NonNegF64;
#[cfg(feature = "alloc")]
use search_core::{BigInt, BigUint};
use search_core::{Decimal, Lexicographic, Status, Unlimited, Zero};

/// Compiles only if `T` is `Send + Sync`.
const fn assert_send_sync<T: Send + Sync>() {}

/// Checked at compile time; the test only makes the check show up in the test list.
#[test]
fn shared_types_are_send_and_sync() {
    const {
        assert_send_sync::<Decimal<2>>();
        assert_send_sync::<Lexicographic<u32, Decimal<2>>>();
        assert_send_sync::<Zero>();
        assert_send_sync::<Unlimited>();
        assert_send_sync::<Status>();
    }

    #[cfg(feature = "alloc")]
    const {
        assert_send_sync::<BigUint>();
        assert_send_sync::<BigInt>();
    }

    #[cfg(feature = "float")]
    const {
        assert_send_sync::<NonNegF64>();
    }
}
