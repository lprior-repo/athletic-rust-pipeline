use super::sheet::SheetState;
use crate::model::{SheetStats, SourceRecord};
use anyhow::Result;
use calamine::Xlsx;
use std::io::{Read, Seek};

pub(crate) fn visit_sheet<RS, F>(
    workbook: &mut Xlsx<RS>,
    name: &str,
    declared_dimension: Option<String>,
    xml_rows: u64,
    retained_header_bytes: &mut usize,
    callback: &mut F,
) -> Result<SheetStats>
where
    RS: Read + Seek,
    F: FnMut(SourceRecord) -> Result<()>,
{
    let mut reader = workbook.worksheet_cells_reader(name)?;
    let mut state = SheetState::new(name, declared_dimension, xml_rows);
    let mut done = false;
    std::iter::from_fn(|| {
        if done {
            return None;
        }
        match reader.next_cell() {
            Ok(Some(cell)) => Some(Ok(cell)),
            Ok(None) => {
                done = true;
                None
            }
            Err(error) => {
                done = true;
                Some(Err(error))
            }
        }
    })
    .try_for_each(|cell| state.accept(cell?, retained_header_bytes, callback))?;
    state.finish(retained_header_bytes, callback)
}
