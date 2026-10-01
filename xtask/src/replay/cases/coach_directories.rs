use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::coach_directories;
use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    let file = capture.file;
    if file.starts_with("nchsaa_directory_") || file.starts_with("ghsa_directory_") {
        return directory(capture);
    }
    if file.starts_with("nc_staff_summary_") || file.starts_with("in_staff_summary_") {
        return summary(capture);
    }
    if file == "summary_orgid_200_accessdenied.xml" {
        return match coach_directories::parse_summary(capture.body.as_bytes()) {
            Ok(_) => bail!("{file}: the AccessDenied body parsed as a school summary"),
            Err(_) => Ok("access_denied rejected as non-JSON".to_string()),
        };
    }
    unmapped("coach_directories", file)
}

fn directory(capture: &Capture<'_>) -> Result<String> {
    let file = capture.file;
    let page = coach_directories::parse_directory(capture.body.as_bytes())?;
    ensure_rows(file, page.results.len(), "directory schools")?;
    let recorded = (
        capture.recorded("page")?.parse::<usize>()?,
        capture.recorded("total_pages")?.parse::<usize>()?,
        capture.recorded("total_results")?.parse::<usize>()?,
        capture.recorded("schools")?.parse::<usize>()?,
    );
    let parsed = (
        page.current_page,
        page.total_pages,
        page.total_results,
        page.results.len(),
    );
    if recorded != parsed {
        bail!("{file}: the parsed page {parsed:?} disagrees with the fixture record {recorded:?}");
    }
    Ok(format!(
        "directory page={} total_pages={} total_results={} schools={}",
        page.current_page,
        page.total_pages,
        page.total_results,
        page.results.len()
    ))
}

fn summary(capture: &Capture<'_>) -> Result<String> {
    let file = capture.file;
    let summary = coach_directories::parse_summary(capture.body.as_bytes())?;
    let school = capture.recorded("school")?;
    let staff = capture.recorded("staff")?.parse::<usize>()?;
    let teams = capture.recorded("teams")?.parse::<usize>()?;
    let rows = capture.recorded("rows")?.parse::<usize>()?;
    if summary.name != school || summary.staff.len() != staff || summary.teams.len() != teams {
        bail!(
            "{file}: the parsed summary {} staff={} teams={} disagrees with the fixture record",
            summary.name,
            summary.staff.len(),
            summary.teams.len()
        );
    }
    let state = UsJurisdiction::from_code(&summary.state_code).ok_or_else(|| {
        anyhow::anyhow!(
            "{file}: `{}` is not a jurisdiction the census reads",
            summary.state_code
        )
    })?;
    let (_, school_id) = CanonicalSchool::new(state, &summary.name, normalize_name(&summary.name));
    let mapped = coach_directories::coach_entities(
        &summary,
        &school_id,
        &coach_directories::summary_url(&summary.short_code),
        "2026-09-29",
        coach_directories::EmissionScope::Census,
    )?;
    if mapped.coaches.len() != rows {
        bail!(
            "{file}: the summary maps to {} rows, the fixture record holds {rows}",
            mapped.coaches.len()
        );
    }
    Ok(format!(
        "summary school={} staff={} teams={} rows={}",
        summary.name,
        summary.staff.len(),
        summary.teams.len(),
        mapped.coaches.len()
    ))
}
