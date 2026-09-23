//! The WIAA arm: a Hy-Tek, RaceDay, or plain result file, read through the adapter's own reader.
//!
//! Two inputs a result file cannot state about itself are supplied here the way the crate's own
//! tests and `benches/core/fixtures.rs` supply them over these same captures: the format, decided
//! from the file's extension and body by the adapter's classifier, and the season, read from the
//! fixture's own record under `tests/golden/`.

use crate::replay::Capture;
use anyhow::{bail, Context, Result};
use census_crawl::wiaa_results;
use census_domain::model::SourceRef;

/// One WIAA result file: the format from its extension and body, the season from its fixture
/// record, and the body through `wiaa_results::parse_result_body` - the call the crate's own tests
/// and `benches/core/fixtures.rs` make over these same captures.
pub(super) fn result_file(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let extension = file.rsplit('.').next().unwrap_or_default();
    let format = wiaa_results::artifact_format(extension, Some(body));
    let year: i16 = capture
        .recorded("archive_year")?
        .parse()
        .context("the fixture record's `archive_year` is not a season")?;
    let source = SourceRef::new("wiaa_results", None);
    let Some(meet) = wiaa_results::parse_result_body(body.as_bytes(), format, source, year) else {
        bail!(
            "{file}: the {} reader published no meet: the body did not parse",
            format.as_str()
        );
    };
    Ok(format!(
        "{} year={year} meet={:?} date={} events={} rows={} skipped={}",
        format.as_str(),
        meet.name,
        meet.date,
        meet.events.len(),
        meet.rows_parsed,
        meet.rows_skipped
    ))
}
