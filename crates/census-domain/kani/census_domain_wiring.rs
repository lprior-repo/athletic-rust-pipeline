// Kani harness wiring for census-domain proof harnesses.
// Included from lib.rs via `#[cfg(kani)] include!("../kani/census_domain_wiring.rs");`
// Paths resolve relative to lib.rs (the including file).

#[path = "gradyear.rs"]
mod kani_gradyear;

#[path = "publish.rs"]
mod kani_publish;

#[path = "id_mint.rs"]
mod kani_id_mint;
