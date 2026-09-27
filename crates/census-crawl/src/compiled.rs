
mod events;
mod header;
mod layout;
mod parse;
mod rows;

pub use crate::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
pub use parse::parse;

#[cfg(test)]
mod tests;
