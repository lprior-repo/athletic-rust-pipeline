//! Timer exports of the "Compiled" family — the format WIAA publishes for regional, sectional and
//! state track meets when no Hy-Tek report is posted.
//!
//! Two events are printed side by side, each in its own block, and every block repeats the same
//! three shapes:
//!
//! ```text
//! Girls' 4x800 Relay Division 1          Finals        Girls' 100 Meters Division 1          Prelims
//!        Team      Relay     Finals  Pts                     Athlete      Yr Team          Prelims
//! 1      HORTONVILLE 'A'     9:55.11  10            1   Parrish, Ashley   11 APPLETON NOR…  12.30 Q
//!     1) Wloszczynski, Lexi 10     2) Young, Ellie 9
//! ```
//!
//! The blocks are found from the anchors of the event header line, the columns from the anchors of
//! the column header line beneath it — the same rule the Hy-Tek parser applies, because a timer
//! export states its layout through label positions and nothing else. Grades are published twice:
//! as the `Yr` column for individuals and per leg for relay members.
//!
//! Layout: `layout` places the blocks and their columns, `events` reads what an event header names,
//! `header` reads the meet name and date, `rows` reads a line's rows and relay legs, and `parse`
//! drives the page.

mod events;
mod header;
mod layout;
mod parse;
mod rows;

pub use crate::sources::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
pub use parse::parse;

#[cfg(test)]
mod tests;
