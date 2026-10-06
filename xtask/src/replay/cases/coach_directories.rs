use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::coach_directories;
use census_domain::model::{normalize_name, CanonicalSchool, SchoolYear, Sport};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn capture_paths(dir: &Path) -> Vec<PathBuf> {
    [
        "nchsaa_directory_p1.json",
        "ghsa_directory_p2.json",
        "nc_staff_summary_zcum49.json",
        "in_staff_summary_qwugx2.json",
        "summary_orgid_200_accessdenied.xml",
    ]
    .into_iter()
    .map(|name| dir.join(name))
    .collect()
}

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
    if summary.name != school || summary.staff.len() != staff || summary.teams.len() != teams {
        bail!(
            "{file}: the parsed summary {} staff={} teams={} disagrees with the fixture record",
            summary.name,
            summary.staff.len(),
            summary.teams.len()
        );
    }
    let state = jurisdiction(&summary.state_code, file)?;
    let (_, school_id) = CanonicalSchool::new(
        state,
        &summary.name,
        normalize_name(&summary.name),
        summary.address.city.as_deref(),
    );
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(capture.body.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    let mapped = coach_directories::coach_entities(
        &summary,
        &school_id,
        &coach_directories::summary_url(&summary.short_code),
        "2026-09-29T00:00:00Z",
        SchoolYear::DEFAULT,
        &digest,
    )?;
    let expected = expected_contexts(capture)?;
    let mut contexts: Vec<String> = mapped
        .coaches
        .iter()
        .map(|coach| {
            format!(
                "{}|{}|{}",
                coach.name,
                coach.sport.map_or("none", Sport::stable_key),
                coach.gender.stable_key()
            )
        })
        .collect();
    contexts.sort_unstable();
    if contexts != expected {
        bail!("{file}: mapped coach contexts {contexts:?} disagree with current qualification {expected:?}");
    }
    Ok(format!(
        "summary school={} staff={} teams={} rows={}",
        summary.name,
        summary.staff.len(),
        summary.teams.len(),
        mapped.coaches.len()
    ))
}

fn jurisdiction(code: &str, file: &str) -> Result<UsJurisdiction> {
    UsJurisdiction::from_code(code)
        .ok_or_else(|| anyhow::anyhow!("{file}: `{}` is not a jurisdiction the census reads", code))
}

fn expected_contexts(capture: &Capture<'_>) -> Result<Vec<String>> {
    let path = capture
        .golden
        .join("coach_directories__census-contexts.json");
    let body = std::fs::read_to_string(&path)?;
    let mut contexts: BTreeMap<String, Vec<String>> = serde_json::from_str(&body)?;
    contexts.remove(capture.file).ok_or_else(|| {
        anyhow::anyhow!(
            "{} has no current coach-context qualification",
            capture.file
        )
    })
}
