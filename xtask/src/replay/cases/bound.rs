use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::bound;

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if !file.starts_with("staff-") || !file.ends_with(".html") {
        return unmapped("bound", file);
    }
    let parsed = bound::parse(body);
    let coaches = parsed.coaches.len();
    if file.contains("empty-staff") {
        if coaches != 0 {
            bail!("{file} listed {coaches} coaches but is the empty-staff capture");
        }
    } else {
        ensure_rows(file, coaches, "coach rows")?;
    }
    Ok(format!(
        "staff school={:?} sport={:?} coaches={coaches}",
        parsed.page.title_school, parsed.page.title_sport
    ))
}
