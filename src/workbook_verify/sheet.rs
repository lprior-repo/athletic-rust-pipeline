use super::headers::parse_headers;
use super::stream::{cell_event, CellRows, Row};
use super::{SheetCounts, MAX_EXCEL_ROW};
use anyhow::{bail, Context, Result};
use calamine::Xlsx;
use std::io;

pub(super) fn verify_sheet<RS, RO>(
    source_book: &mut Xlsx<RS>,
    output_book: &mut Xlsx<RO>,
    name: &str,
    extra_headers: &[String],
    source_header_bytes: &mut usize,
    output_header_bytes: &mut usize,
) -> Result<SheetCounts>
where
    RS: io::Read + io::Seek,
    RO: io::Read + io::Seek,
{
    let mut source_reader = source_book
        .worksheet_cells_reader(name)
        .context("reading source worksheet cells")?;
    let mut output_reader = output_book
        .worksheet_cells_reader(name)
        .context("reading output worksheet cells")?;
    let source_next = move || match source_reader.next_cell() {
        Ok(Some(cell)) => Ok(Some(cell_event(cell)?)),
        Ok(None) => Ok(None),
        Err(error) => Err(error).context("streaming source worksheet cells"),
    };
    let output_next = move || match output_reader.next_cell() {
        Ok(Some(cell)) => Ok(Some(cell_event(cell)?)),
        Ok(None) => Ok(None),
        Err(error) => Err(error).context("streaming output worksheet cells"),
    };
    let mut source_rows = CellRows::new(source_next);
    let mut output_rows = CellRows::new(output_next);
    let source_header = source_rows
        .next()
        .transpose()?
        .context("source worksheet has no nonempty header row")?;
    let output_header = output_rows
        .next()
        .transpose()?
        .context("output worksheet has no nonempty header row")?;
    let source_headers = parse_headers(&source_header, source_header_bytes)?;
    let output_headers = parse_headers(&output_header, output_header_bytes)?;
    if output_headers.len()
        != source_headers
            .len()
            .checked_add(extra_headers.len())
            .context("worksheet header width overflow")?
    {
        bail!("output worksheet header width differs from source plus supplied headers");
    }
    if output_headers.get(..source_headers.len()) != Some(source_headers.as_slice())
        || output_headers.get(source_headers.len()..) != Some(extra_headers)
    {
        bail!("output worksheet headers differ from source headers or supplied headers");
    }
    if extra_headers
        .iter()
        .any(|header| source_headers.iter().any(|source| source == header))
    {
        bail!("supplied extra header collides with a source header");
    }
    let mut counts = SheetCounts {
        source_headers: u64::try_from(source_headers.len())
            .context("source header count conversion overflow")?,
        output_headers: u64::try_from(output_headers.len())
            .context("output header count conversion overflow")?,
        matched_headers: u64::try_from(source_headers.len())
            .context("matched header count conversion overflow")?,
        ..SheetCounts::default()
    };
    loop {
        match (source_rows.next(), output_rows.next()) {
            (Some(source), Some(output)) => {
                let source = source?;
                let output = output?;
                compare_row(
                    &source,
                    &output,
                    source_headers.len(),
                    output_headers.len(),
                    &mut counts,
                )?;
            }
            (Some(source), None) => {
                source?;
                bail!("output worksheet is missing a source data row");
            }
            (None, Some(output)) => {
                output?;
                bail!("output worksheet contains an unexpected data row");
            }
            (None, None) => break,
        }
    }
    Ok(counts)
}

fn compare_row(
    source: &Row,
    output: &Row,
    source_width: usize,
    output_width: usize,
    counts: &mut SheetCounts,
) -> Result<()> {
    if source.number > MAX_EXCEL_ROW
        || output.number > MAX_EXCEL_ROW
        || source.number != output.number
    {
        bail!("worksheet row keys differ or exceed Excel bounds");
    }
    if source.cells.iter().any(|(column, value)| {
        !value.is_empty() && usize::try_from(*column).map_or(true, |index| index >= source_width)
    }) {
        bail!("source worksheet contains an unexpected source field");
    }
    if output
        .cells
        .iter()
        .any(|(column, _)| usize::try_from(*column).map_or(true, |index| index >= output_width))
    {
        bail!("output worksheet contains a field outside its declared headers");
    }
    counts.source_rows = counts
        .source_rows
        .checked_add(1)
        .context("source row count overflow")?;
    counts.output_rows = counts
        .output_rows
        .checked_add(1)
        .context("output row count overflow")?;
    counts.matched_rows = counts
        .matched_rows
        .checked_add(1)
        .context("matched row count overflow")?;
    let width = u64::try_from(source_width).context("source width conversion overflow")?;
    counts.source_fields = counts
        .source_fields
        .checked_add(width)
        .context("source field count overflow")?;
    counts.output_fields = counts
        .output_fields
        .checked_add(width)
        .context("output field count overflow")?;
    counts.matched_fields = counts
        .matched_fields
        .checked_add(width)
        .context("matched field count overflow")?;
    let columns = u32::try_from(source_width).context("source field column count overflow")?;
    (0..columns).try_for_each(|column| {
        let expected = source.cells.get(&column).map_or("", String::as_str);
        let actual = output.cells.get(&column).map_or("", String::as_str);
        if expected == actual {
            Ok(())
        } else {
            bail!("source field differs at worksheet row and column")
        }
    })
}
