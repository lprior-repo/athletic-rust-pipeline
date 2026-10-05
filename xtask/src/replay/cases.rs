mod coach_directories;
mod pa_piaa;
mod results;

use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{bail, Result};
use census_crawl::{ihsa, ihsa::tournament, ks, mshsl, ohsaa, plain_names, wayzata, wiaa};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) fn capture_paths(source: &str, dir: &Path) -> Result<Vec<PathBuf>> {
    match source {
        "coach_directories" => Ok(coach_directories::capture_paths(dir)),
        _ => crate::source_fixture::files_under(dir),
    }
}

pub(super) fn replay(capture: &Capture<'_>) -> Result<String> {
    match capture.source {
        "wiaa" => wiaa(capture),
        "mshsl" => mshsl(capture),
        "ohsaa" => ohsaa(capture),
        "plain_names" => plain_names(capture),
        "ihsa" => ihsa(capture),
        "ihsa_tournament" => ihsa_tournament(capture),
        "ks" => ks_directory(capture),
        "wayzata" => wayzata_schedule(capture),
        "coach_directories" => coach_directories::replay(capture),
        "pa_piaa" => pa_piaa::replay(capture),
        "sidearm_staff" => sidearm_staff(capture),
        "milesplit"
        | "athleticlive"
        | "athleticlive_athletes"
        | "athleticnet"
        | "athleticlive_results"
        | "tfrrs"
        | "wiaa_results" => results::replay(capture),
        other => bail!("no replay arm for source `{other}`: xtask/src/replay/cases.rs states them"),
    }
}

fn wiaa(capture: &Capture<'_>) -> Result<String> {
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

fn mshsl(capture: &Capture<'_>) -> Result<String> {
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

fn ohsaa(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let refusal = matches!(
        file,
        "search_no_results.html" | "sports_malformed.html" | "ad_malformed.html"
    );
    if file.starts_with("search_") {
        let rows = ohsaa::parse_search(body).len();
        if !refusal {
            ensure_rows(file, rows, "search rows")?;
        }
        return Ok(format!("search rows={rows}"));
    }
    if file.starts_with("sports_") {
        let rows = ohsaa::parse_sports_table(body).len();
        if !refusal {
            ensure_rows(file, rows, "sport rows")?;
        }
        return Ok(format!("sports rows={rows}"));
    }
    if file.starts_with("ad_") {
        let page = ohsaa::parse_ad_page(body);
        if !refusal {
            ensure_rows(file, page.office_roles.len(), "AD rows")?;
        }
        let director = page
            .director
            .as_ref()
            .map_or("none", |(name, _)| name.as_str());
        return Ok(format!(
            "ad_page director={director:?} office_roles={}",
            page.office_roles.len()
        ));
    }
    unmapped("ohsaa", file)
}

fn plain_names(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file == "nsaa_directory_form.html" {
        let names = plain_names::parse_nsaa_school_names(body)?;
        ensure_rows(file, names.len(), "school names")?;
        return Ok(format!("nsaa_directory_form names={}", names.len()));
    }
    if file.starts_with("nsaa_directory_") || file.starts_with("nsaa_school_get_") {
        let schools = plain_names::parse_nsaa_directory(body)?;
        ensure_rows(file, schools.len(), "directory rows")?;
        return Ok(format!("nsaa_directory schools={}", schools.len()));
    }
    if file == "nd_schools_index.html" {
        let refs = plain_names::parse_nd_school_refs(body)?;
        ensure_rows(file, refs.len(), "school refs")?;
        return Ok(format!("nd_schools_index schools={}", refs.len()));
    }
    if file.starts_with("nd_school_page") {
        let staff = plain_names::parse_nd_staff(body)?;
        let offerings = plain_names::parse_nd_offerings(body)?;
        ensure_rows(file, staff.len(), "staff rows")?;
        return Ok(format!(
            "nd_school_page staff={} offerings={}",
            staff.len(),
            offerings.len()
        ));
    }
    unmapped("plain_names", file)
}

fn ihsa(capture: &Capture<'_>) -> Result<String> {
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

fn ihsa_tournament(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file == "track_field_meets.json" {
        let meets = tournament::parse::parse_meets(body)?;
        ensure_rows(file, meets.len(), "meets")?;
        return Ok(format!("meets rows={}", meets.len()));
    }
    if file == "track_field_2026_boys_events.json" {
        let events = tournament::parse::parse_events(body)?;
        ensure_rows(file, events.data.len(), "events")?;
        return Ok(format!(
            "events meet_id={} counted={} rows={}",
            events.meet_id,
            events.count,
            events.data.len()
        ));
    }
    if file.starts_with("event_") {
        let summary = tournament::parse::parse_summary(body)?;
        return Ok(format!(
            "event_summary event_id={} has_results={} finishers={}",
            summary.event_id,
            summary.has_results,
            summary.finishers.len()
        ));
    }
    if file.starts_with("cc_qualifiers_") {
        if let Some(error) = tournament::parse::parse_error(body) {
            return Ok(format!("cc_qualifiers archive_error={error:?}"));
        }
        let qualifiers = tournament::parse::parse_qualifiers(body)?;
        return Ok(format!(
            "cc_qualifiers tournament_id={} boxes={} teams={} individuals={}",
            qualifiers.tournament_id,
            qualifiers.box_assignments.len(),
            qualifiers.team_qualifiers.len(),
            qualifiers.individual_qualifiers.len()
        ));
    }
    if file == "terms.json" {
        let terms = tournament::parse::parse_terms(body)?;
        ensure_rows(file, terms.terms.len(), "terms")?;
        return Ok(format!(
            "terms current={} terms={}",
            terms.current_term,
            terms.terms.len()
        ));
    }
    unmapped("ihsa_tournament", file)
}

fn ks_directory(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.starts_with("kshsaa_directory_") {
        let records = ks::parse_records(body)?;
        ensure_rows(file, records.len(), "school records")?;
        return Ok(format!("directory records={}", records.len()));
    }
    unmapped("ks", file)
}

fn sidearm_staff(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.starts_with("gomats.org__staff-directory") {
        let directory = census_crawl::sidearm_staff::parse_staff_directory(body)?;
        ensure_rows(file, directory.members.len(), "staff rows")?;
        let published_emails = directory
            .members
            .iter()
            .filter(|member| !member.email.is_empty())
            .count();
        return Ok(format!(
            "staff_directory name={:?} members={} published_emails={published_emails}",
            directory.name,
            directory.members.len()
        ));
    }
    unmapped("sidearm_staff", file)
}

fn wayzata_schedule(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let Some(year) = season_year(file) else {
        return unmapped("wayzata", file);
    };
    let rows = wayzata::schedule_rows(body, year)?;
    ensure_rows(file, rows.len(), "schedule rows")?;
    Ok(format!("schedule season={year} rows={}", rows.len()))
}

fn season_year(file: &str) -> Option<i16> {
    file.split(|ch: char| !ch.is_ascii_digit())
        .find(|part| part.len() == 4)
        .and_then(|part| part.parse().ok())
}
