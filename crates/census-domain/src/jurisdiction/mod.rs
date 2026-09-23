//! US jurisdictions: the fifty states and the District of Columbia.
//!
//! The census is national, so a jurisdiction is a *validated domain value* rather than a free-form
//! state string. Every adapter, workflow identity and Fjall key that needs a state carries a
//! [`UsJurisdiction`]; the only place a raw string is acceptable is the parse boundary itself.
//!
//! Territories (PR, GU, VI, AS, MP) and freely associated states are deliberately absent: parsing
//! `"PR"` fails instead of silently widening the census, so bringing one in later is an explicit
//! domain change — a new variant with its code and name in [`UsJurisdiction::ALL`] — and cannot
//! happen by accident of string handling. The cohort is high-school TF/XC, and the territorial
//! associations publish under different systems; admitting them silently would corrupt coverage
//! denominators, which is the one number the census is judged on.
//!
//! The two lookup functions are data tables, not logic: a 51-arm constant match is the fastest
//! correct form for a value that ends up in every store key, and the tables stay whole by type —
//! the code/name tables and the 51-variant `ALL` are data, not logic.

pub mod bucket;
pub mod codes;
pub mod meet_state;
pub mod scope;
pub mod ser_de;
pub mod table;

pub use bucket::JurisdictionBucket;
pub use meet_state::MeetState;
pub use scope::OutsideCensusScope;
pub use table::UsJurisdiction;

#[cfg(test)]
mod tests;
