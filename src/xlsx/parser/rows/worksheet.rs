//! The worksheet event loop: one Quick-XML reader over a worksheet part, dispatching every event
//! into the row/cell state machine and closing the document's boundary checks.

use super::super::super::xml::XmlDocument;
use super::state::WorksheetState;
use crate::model::{SheetStats, SourceRecord};
use anyhow::Result;
use quick_xml::{events::Event, Reader};
use std::io::{BufReader, Read};

pub(crate) fn parse_worksheet<R, F>(
    source: R,
    sheet_name: &str,
    shared_strings: &[String],
    mut on_record: F,
) -> Result<SheetStats>
where
    R: Read,
    F: FnMut(SourceRecord) -> Result<()>,
{
    let mut reader = Reader::from_reader(BufReader::new(source));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut document = XmlDocument::default();
    let mut state = WorksheetState::new(sheet_name);
    loop {
        let event = reader.read_event_into(&mut buffer)?;
        state.handle(&event, shared_strings, &mut on_record)?;
        let eof = matches!(&event, Event::Eof);
        document.observe(&event)?;
        buffer.clear();
        if eof {
            break;
        }
    }
    document.finish("worksheet")?;
    state.finish()
}
