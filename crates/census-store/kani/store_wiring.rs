// Kani harness wiring for store-level tests (keys + merge idempotency).
// Included from lib.rs via `#[cfg(kani)] include!("../kani/store_wiring.rs");`
// Paths resolve relative to the including file (src/lib.rs), and the modules below are children of
// the crate root, which is where `keys`' `pub(super)` helpers and `Entity` are visible.

#[path = "../kani/keys.rs"]
mod kani;

#[path = "../kani/merge.rs"]
mod kani_merge;
