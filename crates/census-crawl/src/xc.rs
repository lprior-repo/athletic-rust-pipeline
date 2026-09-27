
mod header;
mod parse;
mod patterns;
mod rows;
mod scan;

pub use crate::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
pub use parse::parse;

#[cfg(test)]
use crate::CrawlError;
#[cfg(test)]
use census_domain::model::{CentiSeconds, EventKind, Gender, Mark, SourceRef};

#[cfg(test)]
mod tests;
