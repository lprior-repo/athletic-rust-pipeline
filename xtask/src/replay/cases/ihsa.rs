use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;
use census_crawl::ihsa;
use std::collections::BTreeSet;

pub(super) fn ihsa(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file == "v1_schools.json" {
        let schools = ihsa::parse_schools(body)?;
        ensure_rows(file, schools.len(), "school rows")?;
        return Ok(format!("schools rows={}", schools.len()));
    }
    if file.starts_with("staff2_") {
        let staff = ihsa::parse_staff(body)?;
        ensure_rows(file, staff.len(), "staff rows")?;
        let people: BTreeSet<i64> = staff.iter().map(|row| row.person_id).collect();
        return Ok(format!(
            "staff rows={} people={}",
            staff.len(),
            people.len()
        ));
    }
    unmapped("ihsa", file)
}
