//! Worksheet row extraction: the streaming reader for one `xl/worksheets/*.xml` part.
//!
//! Split into a directory module when this file outgrew the repository's 300-line budget; the
//! reader entry point stays re-exported here for `parser`, the event loop lives in `worksheet`, the
//! row/cell state machine in `state`, row numbering in `position`, and cell placement with
//! shared-string resolution in `cell`.

mod cell;
mod position;
mod state;
mod worksheet;

pub(crate) use worksheet::parse_worksheet;
