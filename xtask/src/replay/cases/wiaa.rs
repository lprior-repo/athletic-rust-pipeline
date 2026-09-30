use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::wiaa;

pub(super) fn wiaa(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.starts_with("directory_letter_") {
        let entries = wiaa::parse_directory_letter(body);
        ensure_rows(file, entries.len(), "index rows")?;
        return Ok(format!("directory_letter rows={}", entries.len()));
    }
    if file.starts_with("school_org") {
        let page = wiaa::parse_school_page(body);
        if page.name.is_empty() {
            bail!("{file} yielded no school name: the body did not parse");
        }
        return Ok(format!(
            "school_page name={:?} enrollment={:?} admins={} coaches={}",
            page.name,
            page.enrollment,
            page.admins.len(),
            page.coaches.len()
        ));
    }
    unmapped("wiaa", file)
}
