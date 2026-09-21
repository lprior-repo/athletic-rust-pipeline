//! Cross-country result files published by WIAA's timers.
//!
//! Three layouts appear on the archive, all of them carrying the runner's grade — which is what
//! makes cross-country a class-of-2027 source rather than only a results source:
//!
//! ```text
//! 1.  Hy-Tek team blocks (state meet, several sectionals)
//!     1.    69 SPASH                             (16:09.3 80:46.1 0:43.4)
//!       1      6 Cooper Erickson   12 15:50.2
//!
//! 2.  Padded grade table (several sectionals)
//!     Place   Points   Bib   Name        School        Gender   Grade   Time      Pace
//!     1       1        574   Wyatt See   Poynette      M        12      16:44.1   5:23
//!
//! 3.  AccuRace Timing's rule-lined table
//!          Team Team                                      Avg   State
//!     Place Pts Place Bib#   Name        Gr   Team        Time  Mile Qual
//!     ===== ==== ===== ====   ========== ==   =========== ===== ===== =====
//!         1    1 1/7 8223     Jonathan Simon 10  St. Ambrose/Abundant Life 16:21.6 5:16 t
//! ```
//!
//! Rows carry a team label, a place, a time and a grade; the canonical meet name and date come from
//! the file's own header lines.

mod header;
mod parse;
mod patterns;
mod rows;
mod scan;

pub use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
pub use parse::parse;

// The moved bodies reach these through `super::*`; the test module is the only reader.
#[cfg(test)]
use crate::sources::{CrawlError, CrawlResult};
#[cfg(test)]
use census_domain::model::{EventKind, Gender, Mark, SourceRef};

#[cfg(test)]
mod tests;
