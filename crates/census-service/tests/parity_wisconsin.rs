mod common;
#[path = "common/listing.rs"]
mod listing;

use anyhow::{ensure, Context, Result};
use census_crawl::{hytek, xc};
use census_domain::model::SourceRef;

const ARCHIVE_YEARS: [(&str, i16); 6] = [
    ("d1boysstateresults-dash.htm", 2025),
    ("d1boysstateresults-dash.txt", 2025),
    ("d1boysstateresults-sections.htm", 2025),
    ("racinesectionalb-finish-list.htm", 2023),
    ("seed-column-regional.htm", 2025),
    ("trackside-regional.htm", 2025),
];

#[test]
fn xc_declines_every_track_and_field_archive_fixture() -> Result<()> {
    for path in listing::fixtures("wiaa_results")? {
        let file = listing::file_name(&path)?;
        let body = common::fixture("wiaa_results", &file)?;
        let year = ARCHIVE_YEARS
            .iter()
            .find(|(name, _)| *name == file)
            .map(|(_, year)| *year)
            .with_context(|| format!("no archive year for {file}"))?;
        let lines = if file
            .rsplit_once('.')
            .is_some_and(|(_, extension)| extension == "txt")
        {
            hytek::lines_from_text(&body)
        } else {
            hytek::lines_from_html(&body)
        };
        ensure!(
            xc::parse(&lines, SourceRef::new("wiaa_results", None), year).is_none(),
            "{file} is a Track & Field report incorrectly claimed by the cross-country parser"
        );
    }
    Ok(())
}
