use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::mshsl;

pub(super) fn mshsl(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.starts_with("schools_listing") {
        let schools = mshsl::parse_school_list(body);
        ensure_rows(file, schools.len(), "listing rows")?;
        let next = mshsl::parse_next_listing_page(body, 0);
        return Ok(format!(
            "school_listing schools={} next_page={next:?}",
            schools.len()
        ));
    }
    if file.starts_with("school_detail_") {
        let detail = mshsl::parse_school_detail(body);
        let Some(name) = detail.name.as_deref().filter(|name| !name.is_empty()) else {
            bail!("{file} yielded no school name: the body did not parse");
        };
        return Ok(format!(
            "school_detail name={name:?} school_id={:?} enrollment={:?} admin={}",
            detail.school_id,
            detail.enrollment,
            detail.admin.len()
        ));
    }
    if file.starts_with("team_nodes_") {
        let nodes = mshsl::parse_team_nodes(body);
        ensure_rows(file, nodes.len(), "team nodes")?;
        let selected = mshsl::select_team_nodes(&nodes);
        return Ok(format!(
            "team_nodes nodes={} selected={}",
            nodes.len(),
            selected.len()
        ));
    }
    if file.starts_with("coach_records_") {
        let records = mshsl::parse_coach_records(body);
        ensure_rows(file, records.len(), "coach records")?;
        let named = records
            .iter()
            .filter(|record| !record.name.trim().is_empty())
            .count();
        return Ok(format!(
            "coach_records rows={} named={named}",
            records.len()
        ));
    }
    unmapped("mshsl", file)
}
