//! AthleticLIVE meet import (research-artifact adapter).
//!
//! Consumes the AthleticLIVE tenant/meet harvest produced by the source-research phase
//! (`athleticlive-midwest-2026-meets-all.csv`, one row per tenant x meet) and turns it into
//! canonical meets.
//!
//! Why this matters: 84-89% of AthleticLIVE meet documents carry an Athletic.net meet id. That
//! makes this artifact a *bridge*: it names specific Athletic.net meets (date, venue, id) without
//! issuing any Athletic.net request, so later targeted acquisition can address one meet directly
//! instead of enumerating Athletic.net's meet universe.
//!
//! This adapter performs no HTTP. `Options::input` MUST point at the CSV; the research corpus is
//! the source of record and is never edited here.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context, Result};

use crate::model::{
    CanonicalMeet, CompetitionLevel, Evidence, EvidenceMethod, MeetId, SourceIdentity,
    SourceNamespace, SourceRef,
};
use crate::sources::{AdapterContext, AdapterReport};

/// Adapter options (uniform across provider adapters plus `input`).
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Path to `athleticlive-midwest-2026-meets-all.csv` (or the `-seeds` variant).
    pub input: Option<String>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to these state codes; empty = every state present in the file.
    pub states: Vec<String>,
    pub school_names: Vec<String>,
}

impl Options {
    /// Options for the research-corpus artifact, stamped with an observation date.
    pub fn for_input(input: impl Into<String>, observed_on: impl Into<String>) -> Self {
        Self {
            input: Some(input.into()),
            observed_on: observed_on.into(),
            ..Default::default()
        }
    }
}

/// One parsed row of the AthleticLIVE harvest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRow {
    pub tenant: String,
    pub athleticlive_meet_id: String,
    pub athleticnet_meet_id: Option<String>,
    pub name: String,
    pub city_state: Option<String>,
    pub state_code: String,
    pub start: String,
    pub end: Option<String>,
    pub has_results: bool,
}

/// Map a full US state name (as published by AthleticLIVE) to its postal code.
pub fn state_code(name: &str) -> Option<&'static str> {
    const STATES: [(&str, &str); 51] = [
        ("Alabama", "AL"),
        ("Alaska", "AK"),
        ("Arizona", "AZ"),
        ("Arkansas", "AR"),
        ("California", "CA"),
        ("Colorado", "CO"),
        ("Connecticut", "CT"),
        ("Delaware", "DE"),
        ("District of Columbia", "DC"),
        ("Florida", "FL"),
        ("Georgia", "GA"),
        ("Hawaii", "HI"),
        ("Idaho", "ID"),
        ("Illinois", "IL"),
        ("Indiana", "IN"),
        ("Iowa", "IA"),
        ("Kansas", "KS"),
        ("Kentucky", "KY"),
        ("Louisiana", "LA"),
        ("Maine", "ME"),
        ("Maryland", "MD"),
        ("Massachusetts", "MA"),
        ("Michigan", "MI"),
        ("Minnesota", "MN"),
        ("Mississippi", "MS"),
        ("Missouri", "MO"),
        ("Montana", "MT"),
        ("Nebraska", "NE"),
        ("Nevada", "NV"),
        ("New Hampshire", "NH"),
        ("New Jersey", "NJ"),
        ("New Mexico", "NM"),
        ("New York", "NY"),
        ("North Carolina", "NC"),
        ("North Dakota", "ND"),
        ("Ohio", "OH"),
        ("Oklahoma", "OK"),
        ("Oregon", "OR"),
        ("Pennsylvania", "PA"),
        ("Rhode Island", "RI"),
        ("South Carolina", "SC"),
        ("South Dakota", "SD"),
        ("Tennessee", "TN"),
        ("Texas", "TX"),
        ("Utah", "UT"),
        ("Vermont", "VT"),
        ("Virginia", "VA"),
        ("Washington", "WA"),
        ("West Virginia", "WV"),
        ("Wisconsin", "WI"),
        ("Wyoming", "WY"),
    ];
    let trimmed = name.trim();
    STATES
        .iter()
        .find(|(full, code)| {
            full.eq_ignore_ascii_case(trimmed) || code.eq_ignore_ascii_case(trimmed)
        })
        .map(|(_, code)| *code)
}

/// True when a date's year is outside a plausible high-school competition window.
///
/// The harvest contains a small tail of corrupt timestamps (for example `2222-08-23`); they are
/// imported as published and counted in the report rather than dropped, so the corruption stays
/// visible downstream.
pub fn implausible_year(date: &str) -> bool {
    match date.get(..4).and_then(|y| y.parse::<u16>().ok()) {
        Some(year) => !(2015..=2030).contains(&year),
        None => true,
    }
}

/// Take the `YYYY-MM-DD` prefix of an ISO timestamp; reject anything else.
fn date_prefix(value: &str) -> Option<String> {
    let head = value.trim().get(..10)?;
    let mut parts = head.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    if !y
        .chars()
        .chain(m.chars())
        .chain(d.chars())
        .all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some(head.to_string())
}

/// Competition level inferred from meet-name vocabulary.
///
/// Deliberately shallow: only unambiguous markers are honoured, everything else stays `Unknown`
/// rather than guessing. The meet name remains the primary evidence for a later cross-source join.
pub fn infer_level(name: &str) -> CompetitionLevel {
    let lowered = name.to_ascii_lowercase();
    let has = |needle: &str| lowered.contains(needle);
    if has("state") && (has("final") || has("championship") || has("champ") || has("meet")) {
        CompetitionLevel::State
    } else if has("sectional") {
        CompetitionLevel::Sectional
    } else if has("regional") {
        CompetitionLevel::Regional
    } else if has("district") {
        CompetitionLevel::District
    } else if has("conference") || has("conf ") {
        CompetitionLevel::Conference
    } else if has("invitational")
        || has("invite")
        || has("classic")
        || has("relays")
        || has("festival")
        || has("open")
    {
        CompetitionLevel::Invitational
    } else if has("dual") {
        CompetitionLevel::Dual
    } else {
        CompetitionLevel::Unknown
    }
}

/// Parse the AthleticLIVE harvest CSV. Column order is irrelevant; unknown columns are ignored and
/// the optional columns of the `-seeds` variant are tolerated.
pub fn parse_meets_csv(body: &str) -> Result<Vec<MeetRow>> {
    let mut lines = body.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().context("AthleticLIVE CSV has no header row")?;
    let columns = split_csv_record(header);
    let index_of = |want: &str| columns.iter().position(|c| c.eq_ignore_ascii_case(want));
    for required in ["tenant", "athleticlive_meet_id", "name", "state", "start"] {
        if index_of(required).is_none() {
            bail!("AthleticLIVE CSV is missing required column `{required}`");
        }
    }

    let mut rows = Vec::new();
    for (n, line) in lines.enumerate() {
        let fields = split_csv_record(line);
        let get = |want: &str| index_of(want).and_then(|i| fields.get(i)).map(|s| s.trim());
        let tenant = match get("tenant") {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => bail!("row {} has no tenant", n + 2),
        };
        let meet_id = match get("athleticlive_meet_id") {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => bail!("row {} has no athleticlive_meet_id", n + 2),
        };
        let name = match get("name") {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => bail!("row {} has no name", n + 2),
        };
        let state_raw = get("state").unwrap_or("");
        let Some(state_code) = state_code(state_raw) else {
            continue;
        };
        let Some(start) = get("start").and_then(date_prefix) else {
            continue;
        };
        let end = get("end").and_then(date_prefix).filter(|e| e != &start);
        let athleticnet_meet_id = get("athleticnet_meet_id")
            .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
            .map(str::to_string);
        let city_state = get("city_state")
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let has_results = get("has_results")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        rows.push(MeetRow {
            tenant,
            athleticlive_meet_id: meet_id,
            athleticnet_meet_id,
            name,
            city_state,
            state_code: state_code.to_string(),
            start,
            end,
            has_results,
        });
    }
    Ok(rows)
}

/// Minimal RFC 4180 record splitter (the harvest quotes timer-credit HTML, so this is required).
fn split_csv_record(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => fields.push(std::mem::take(&mut current)),
            other => current.push(other),
        }
    }
    fields.push(current);
    fields
}

/// Build canonical meets from parsed rows, merging tenants that publish the same meet.
pub fn build_meets(rows: &[MeetRow], observed_on: &str, source_label: &str) -> Vec<CanonicalMeet> {
    let mut meets: BTreeMap<MeetId, CanonicalMeet> = BTreeMap::new();
    for row in rows {
        let id = CanonicalMeet::mint(
            &row.state_code,
            &row.start,
            &row.name,
            row.city_state.as_deref(),
        );
        let entry = meets.entry(id.clone()).or_insert_with(|| {
            let mut meet = CanonicalMeet::new(
                &row.state_code,
                row.name.clone(),
                row.start.clone(),
                infer_level(&row.name),
            );
            meet.end_date = row.end.clone();
            meet.location = row.city_state.clone();
            meet.evidence.push(Evidence::parsed(
                SourceRef::new(source_label, None),
                observed_on,
            ));
            meet
        });
        if entry.end_date.is_none() {
            entry.end_date = row.end.clone();
        }
        if entry.location.is_none() {
            entry.location = row.city_state.clone();
        }
        let timer_identity = SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: row.tenant.clone(),
            },
            row.athleticlive_meet_id.clone(),
        );
        if !entry
            .source_identities
            .iter()
            .any(|existing| existing == &timer_identity)
        {
            entry.source_identities.push(timer_identity);
        }
        if let Some(an_id) = &row.athleticnet_meet_id {
            let an_identity = SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "meet".to_string(),
                },
                an_id.clone(),
            )
            .with_url(format!(
                "https://www.athletic.net/TrackAndField/meet/{an_id}/info"
            ));
            if !entry
                .source_identities
                .iter()
                .any(|existing| existing == &an_identity)
            {
                entry.source_identities.push(an_identity);
            }
        }
    }
    meets.into_values().collect()
}

/// Import AthleticLIVE meets into the canonical store.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let Some(input) = options.input.as_deref() else {
        bail!("athleticlive adapter requires --input <path to athleticlive-midwest-2026-meets-all.csv>");
    };
    let body = std::fs::read_to_string(input)
        .with_context(|| format!("reading AthleticLIVE harvest {input}"))?;
    let mut report = AdapterReport::new("athleticlive", "meets");
    let parsed = parse_meets_csv(&body)?;
    report.note(format!("rows read: {}", parsed.len()));

    let wanted: BTreeSet<String> = options
        .states
        .iter()
        .filter_map(|s| state_code(s).map(str::to_string))
        .collect();
    let filtered: Vec<MeetRow> = parsed
        .into_iter()
        .filter(|row| wanted.is_empty() || wanted.contains(&row.state_code))
        .collect();

    let mut meets = build_meets(&filtered, &options.observed_on, "athleticlive_meets_csv");
    if let Some(limit) = options.limit {
        meets.truncate(limit);
    }

    let done = ctx.store.journal_keys("athleticlive_meets")?;
    let mut written = 0usize;
    let mut skipped_corrupt = 0usize;
    for meet in meets {
        if done.contains(meet.id.as_str()) {
            continue;
        }
        // Placeholder rows in the tenant index carry impossible dates (`2222-08-23`). A meet is
        // the anchor for the school year a grade is read against, so an impossible date would
        // mint an impossible graduating class downstream: refuse it here instead.
        if implausible_year(&meet.date) {
            skipped_corrupt += 1;
            continue;
        }
        ctx.store.append(crate::store::Table::Meets, &meet)?;
        ctx.store.journal_done(
            "athleticlive_meets",
            meet.id.as_str(),
            &serde_json::json!({ "state": meet.state, "date": meet.date }),
        )?;
        written += 1;
    }

    let tenants: BTreeSet<&str> = filtered.iter().map(|r| r.tenant.as_str()).collect();
    let with_an = filtered
        .iter()
        .filter(|r| r.athleticnet_meet_id.is_some())
        .count();
    let corrupt = filtered
        .iter()
        .filter(|r| implausible_year(&r.start))
        .count();
    report.note(format!(
        "rows with implausible years (outside 2015-2030): {corrupt} (refused at write; skipped this run: {skipped_corrupt})"
    ));
    let min_date = filtered.iter().map(|r| r.start.as_str()).min();
    let max_date = filtered.iter().map(|r| r.start.as_str()).max();
    report.rows = written as u64;
    report.note(format!("distinct tenants: {}", tenants.len()));
    report.note(format!(
        "rows with an Athletic.net meet id: {with_an}/{}",
        filtered.len()
    ));
    report.note(format!(
        "meets written: {written} (journal had {})",
        done.len()
    ));
    if let (Some(min), Some(max)) = (min_date, max_date) {
        report.note(format!("date span: {min}..{max}"));
    }
    let observed = EvidenceMethod::Parsed;
    report.note(format!("evidence method: {observed:?}"));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HARVEST: &str = include_str!("../../tests/fixtures/athleticlive/meets-sample.csv");

    #[test]
    fn parses_harvest_rows_and_keeps_only_known_states() {
        let rows = parse_meets_csv(HARVEST).expect("fixture parses");
        assert!(rows.len() >= 4, "fixture should carry several rows");
        assert!(
            rows.iter().all(|r| !r.name.is_empty()),
            "every kept row has a name"
        );
        assert!(rows.iter().any(|r| r.state_code == "IL"));
        assert!(
            rows.iter().any(|r| r.athleticnet_meet_id.is_some()),
            "the harvest carries Athletic.net meet ids"
        );
    }

    #[test]
    fn quoted_fields_do_not_break_column_alignment() {
        let csv = "tenant,athleticlive_meet_id,athleticnet_meet_id,name,city_state,state,start,end,has_results,timer_credit\n\
                   palatine,55274,259955,\"4th Annual St. Pius X, Knights Classic\",\"Lombard, IL\",Illinois,2025-08-16T04:00:00Z,,True,\"Timed by <a href=\"\"x\"\">Palatine Pack</a>\"\n";
        let rows = parse_meets_csv(csv).expect("quoted line parses");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "4th Annual St. Pius X, Knights Classic");
        assert_eq!(rows[0].city_state.as_deref(), Some("Lombard, IL"));
        assert_eq!(rows[0].state_code, "IL");
        assert_eq!(rows[0].athleticnet_meet_id.as_deref(), Some("259955"));
        assert!(rows[0].has_results);
    }

    #[test]
    fn missing_required_column_is_an_error_not_a_panic() {
        let err =
            parse_meets_csv("tenant,name,state,start\nx,Meet,Illinois,2025-08-16T04:00:00Z\n")
                .expect_err("athleticlive_meet_id is required");
        assert!(err.to_string().contains("athleticlive_meet_id"));
    }

    #[test]
    fn corrupt_years_are_flagged_not_silently_kept() {
        assert!(!implausible_year("2026-05-29"));
        assert!(!implausible_year("2015-01-01"));
        assert!(implausible_year("2222-08-23"));
        assert!(implausible_year("not-a-date"));
        let rows = parse_meets_csv(HARVEST).expect("fixture parses");
        assert!(rows.iter().all(|r| !implausible_year(&r.start)));
    }

    #[test]
    fn level_inference_only_fires_on_explicit_markers() {
        assert_eq!(
            infer_level("WIAA State Championships"),
            CompetitionLevel::State
        );
        assert_eq!(infer_level("D3 Sectional #3"), CompetitionLevel::Sectional);
        assert_eq!(
            infer_level("Big Rivers Conference Meet"),
            CompetitionLevel::Conference
        );
        assert_eq!(
            infer_level("Knights Chicagoland Classic"),
            CompetitionLevel::Invitational
        );
        assert_eq!(infer_level("Weekend Race #4"), CompetitionLevel::Unknown);
    }

    #[test]
    fn tenants_publishing_one_meet_merge_into_one_canonical_meet() {
        let rows = parse_meets_csv(HARVEST).expect("fixture parses");
        let meets = build_meets(&rows, "2026-09-20", "athleticlive_meets_csv");
        let classic = meets
            .iter()
            .find(|m| m.name.contains("Knights Chicagoland"))
            .expect("fixture meet present");
        let timers: Vec<&SourceIdentity> = classic
            .source_identities
            .iter()
            .filter(|i| matches!(i.namespace, SourceNamespace::TimerMeet { .. }))
            .collect();
        assert!(
            timers.len() >= 2,
            "both tenants are recorded on one canonical meet"
        );
        assert_eq!(timers[0].id, "55274");
        let an: Vec<&SourceIdentity> = classic
            .source_identities
            .iter()
            .filter(|i| matches!(i.namespace, SourceNamespace::LegacyAthleticNet { .. }))
            .collect();
        assert_eq!(an.len(), 1, "the Athletic.net meet id is deduplicated");
        assert_eq!(an[0].id, "259955");
        assert_eq!(
            an[0].url.as_deref(),
            Some("https://www.athletic.net/TrackAndField/meet/259955/info")
        );
        assert_eq!(classic.location.as_deref(), Some("Lombard, IL"));
        assert_eq!(classic.date, "2025-08-16");
        assert_eq!(meets.iter().filter(|m| m.id == classic.id).count(), 1);
    }
}
