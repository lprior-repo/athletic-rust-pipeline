// Kani harness wiring for store-level tests (keys + merge idempotency).
// Included from store/mod.rs via `#[cfg(kani)] include!("../kani/store_wiring.rs");`
// Paths resolve relative to store/mod.rs (the including file).

#[path = "../kani/keys.rs"]
mod kani;

#[path = "../kani/merge.rs"]
mod kani_merge;
