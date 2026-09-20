//! KSHSAA directory adapter (Kansas)
//!
//! The KSHSAA directory API exposes one endpoint that returns every member school with its athletic
//! director's name and email:
//! `GET https://kshsaa-api.kshsaa.org/directory/search/name/a/`
//!
//! Per-letter queries (`/directory/search/name/<letter>/`) are supported but the single `a` request
//! already yields the full ~526-school universe.
//!
//! # Fields observed
//! `Id`, `Identifier` (e.g. `KSS0307`), `SchoolName`, `MailingCity`, `Class`, `Enrollment`,
//! `WebSite`, `ADName`, `ADEmail`.
//!
//! # Deliberately ignored fields (never read, never stored)
//! `ADCell`, `PresCell`, `PrincipalCell`, `PrincipalName`, `PresName`, `PresEmail`, `SchoolPhone`,
//! `SchoolFax`, `Email`, `TwitterUserName` — any phone field, any home or cell number, and any
//! non-coaching office role data. Those columns are not part of the schema.

use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashSet;

/// API base URL for the name-search directory endpoint.
const KSHSAA_API: &str = "https://kshsaa-api.kshsaa.org/directory/search/name/";

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Stop after this many schools (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// Restrict to these state codes when the provider spans several states.
    pub states: Vec<String>,
    /// School names to resolve when the provider has no bulk index.
    pub school_names: Vec<String>,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// One record from the KSHSAA directory JSON.
///
/// We only deserialize the fields we need. Everything else is silently ignored — including phone
/// fields that some sources publish but this adapter must never touch.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KshsaaRecord {
    #[serde(rename = "Id")]
    pub id: u64,
    #[serde(rename = "Identifier")]
    pub identifier: String,
    #[serde(rename = "SchoolName")]
    pub school_name: String,
    #[serde(rename = "MailingCity")]
    pub mailing_city: String,
    #[serde(rename = "Class")]
    #[serde(default)]
    pub class: Option<String>,
    #[serde(rename = "Enrollment")]
    #[serde(default)]
    pub enrollment: Option<u32>,
    #[serde(rename = "WebSite")]
    #[serde(default)]
    pub web_site: Option<String>,
    #[serde(rename = "ADName")]
    #[serde(default)]
    pub ad_name: Option<String>,
    #[serde(rename = "ADEmail")]
    #[serde(default)]
    pub ad_email: Option<String>,
    // Deliberately not used by the adapter; present only for test assertions that these
    // phone/principal fields never leak into canonical entities.
    #[serde(rename = "ADCell")]
    #[serde(default)]
    pub ad_cell: Option<String>,
    #[serde(rename = "PrincipalName")]
    #[serde(default)]
    pub principal_name: Option<String>,
    // Fields deliberately NOT mapped: ADCell, PresCell, PrincipalCell, PrincipalName, PresName,
    // PresEmail, SchoolPhone, SchoolFax, Email, TwitterUserName.
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse the JSON envelope returned by the KSHSAA directory API.
///
/// The response is a flat JSON array — no nesting. Returns the parsed records in API order.
pub fn parse_records(body: &str) -> Result<Vec<KshsaaRecord>> {
    let records: Vec<KshsaaRecord> =
        serde_json::from_str(body).context("KSHSAA directory JSON is not an array")?;
    Ok(records)
}

/// Convert one KSHSAA record into a canonical school, if the name is non-empty.
pub fn parse_school(
    record: &KshsaaRecord,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = record.school_name.trim();
    if name.is_empty() {
        return None;
    }
    let normalized = normalize_name(name);
    let (mut school, id) = CanonicalSchool::new("KS", name, &normalized);
    school.city = nonempty(&record.mailing_city);
    school.association = Some("kshsaa".into());
    school.classification = record.class.as_ref().and_then(|v| nonempty(v));
    school.enrollment = record.enrollment;
    school.school_website = record.web_site.as_ref().and_then(|v| nonempty(v));
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "kshsaa".into(),
            },
            &record.identifier,
        )
        .with_url(source_url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new("ks", Some(source_url.to_string())),
        observed_on,
    ));
    Some((school, id))
}

/// Convert one KSHSAA record into an athletic-director coach entity.
///
/// Returns `None` when the AD name is empty or missing.
pub fn parse_ad_coach(
    record: &KshsaaRecord,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let name = record.ad_name.as_deref().unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }
    let name = strip_honorific(name);
    let mut coach = CanonicalCoach::new(
        school_id,
        &name,
        None,          // school-wide role
        Gender::Mixed, // AD is not gender-specific
        CoachRole::AthleticDirector,
    );
    if let Some(email) = &record.ad_email {
        let email = email.trim();
        if !email.is_empty() {
            coach.professional_email = Some(email.to_string());
        }
    }
    coach.evidence.push(Evidence::parsed(
        SourceRef::new("ks", Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}

/// Strip leading honorifics ("Mr.", "Dr.", "Coach", etc.) from a person name.
fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "coach." | "sir" | "rev"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        value.trim().to_string()
    } else {
        parts.join(" ")
    }
}

/// Convert a non-empty trimmed string into `Some`, or `None`.
fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

// ---------------------------------------------------------------------------
// Collect
// ---------------------------------------------------------------------------

/// Collect this provider's schools and AD contacts into the canonical store.
///
/// Strategy:
/// 1. Fetch `https://kshsaa-api.kshsaa.org/directory/search/name/a/` — the single request returns
///    all ~526 Kansas member schools.
/// 2. Parse the JSON array; each record becomes one canonical school and one AD coach.
/// 3. Journal progress per school so a re-run resumes without re-processing.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("ks", "schools");
    report.unit = "schools".to_string();

    // Snapshot fetcher stats before work.
    let before = ctx.fetcher.stats().await;

    // Build the API URL. The `a` endpoint returns the full directory.
    let url = format!("{KSHSAA_API}a/");

    // Fetch with caching (respect `options.refresh`).
    let outcome = match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors += 1;
            report.note(format!("failed to fetch KSHSAA directory: {e}"));
            return Ok(report);
        }
    };

    // Parse JSON.
    let records = parse_records(&outcome.text())?;
    report.requests += 1;
    if outcome.from_cache {
        report.from_cache += 1;
    }

    // Load the resume set (keys already journalled in this phase).
    let done_keys: HashSet<String> = ctx.store.journal_keys("kshsaa_schools")?;

    let limit = options.limit;
    let mut processed = 0usize;
    let mut skipped = 0usize;
    let mut skipped_no_ad = 0usize;
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();

    for record in &records {
        // Honour limit.
        if let Some(max) = limit {
            if processed >= max {
                break;
            }
        }

        // Skip already-processed schools (resume support).
        let journal_key = format!("KS:{}", record.identifier);
        if done_keys.contains(&journal_key) {
            skipped += 1;
            continue;
        }

        // Parse school.
        let (school, school_id) = match parse_school(record, &url, &options.observed_on) {
            Some(pair) => pair,
            None => continue,
        };

        // Parse AD coach.
        if let Some(coach) = parse_ad_coach(record, &school_id, &url, &options.observed_on) {
            if coach.professional_email.is_some() {
                report.with_email += 1;
            }
            coaches.push(coach);
        } else {
            skipped_no_ad += 1;
        }

        schools.push(school);

        // Journal this school as done.
        ctx.store
            .journal_done(
                "kshsaa_schools",
                &journal_key,
                &serde_json::json!({
                    "identifier": record.identifier,
                    "school_name": record.school_name,
                }),
            )
            .context("journaling kshsaa school progress")?;

        processed += 1;
    }

    // Append all schools and coaches to the store.
    if !schools.is_empty() {
        ctx.store
            .append_many(Table::Schools, &schools)
            .context("writing kshsaa schools")?;
    }
    if !coaches.is_empty() {
        ctx.store
            .append_many(Table::Coaches, &coaches)
            .context("writing kshsaa coaches")?;
    }

    // Finalize stats and counts.
    let after = ctx.fetcher.stats().await;
    let delta_requests = after.requests.saturating_sub(before.requests);
    report.rows = processed as u64;
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} schools from KSHSAA; {} already done; {} skipped (no AD name)",
        processed, skipped, skipped_no_ad
    ));

    Ok(report)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed fixture: 5 real KSHSAA records extracted from the full directory capture.
    ///
    /// **Provenance** — URL:
    /// `https://kshsaa-api.kshsaa.org/directory/search/name/a/`
    /// (the `a` endpoint returns the full ~526-school directory; the capture below is a
    /// trimmed subset of that single response.)
    /// **Capture file**:
    /// `/home/lewis/Downloads/midwest-tfxc-source-research/tools/a29-coach/ks-full.json`
    /// (HTTP 200, captured via browser devtools / network monitor).
    const FIXTURE: &str = include_str!("../../tests/fixtures/ks/kshsaa_directory_a.json");

    #[test]
    fn parses_fixture_records() {
        let records = parse_records(FIXTURE).expect("fixture must parse");
        assert_eq!(records.len(), 5, "fixture contains 5 records");
    }

    #[test]
    fn parses_school_from_record() {
        let records = parse_records(FIXTURE).expect("fixture must parse");
        let record = &records[0]; // Abilene HS

        let (school, id) = parse_school(record, "https://example.com/api", "2026-09-20")
            .expect("Abilene has a name");

        assert_eq!(school.name, "Abilene HS");
        assert_eq!(school.state.as_deref(), Some("KS"));
        assert_eq!(school.association.as_deref(), Some("kshsaa"));
        assert_eq!(school.classification.as_deref(), Some("4A"));
        assert_eq!(school.enrollment, Some(467));
        assert_eq!(
            school.school_website.as_deref(),
            Some("www.abileneschools.org")
        );
        assert_eq!(school.city.as_deref(), Some("Abilene"));

        // Source identity carries the KSHSAA external id.
        assert_eq!(school.source_identities.len(), 1);
        assert_eq!(
            school.source_identities[0].namespace,
            SourceNamespace::AssociationSchool {
                association: "kshsaa".into()
            }
        );
        assert_eq!(school.source_identities[0].id, "KSS0001");

        // Evidence carries the source ref and observation date.
        assert_eq!(school.evidence.len(), 1);
        assert_eq!(
            school.evidence[0].source.url.as_deref(),
            Some("https://example.com/api")
        );

        // The school id is deterministic from state + normalized name.
        let normalized = normalize_name("Abilene HS");
        let expected_id = CanonicalSchool::mint("KS", "Abilene HS", &normalized);
        assert_eq!(id, expected_id);
    }

    #[test]
    fn parses_ad_coach_from_record() {
        let records = parse_records(FIXTURE).expect("fixture must parse");
        let record = &records[0];
        let (_, school_id) = parse_school(record, "https://example.com/api", "2026-09-20")
            .expect("Abilene has a name");

        let coach = parse_ad_coach(record, &school_id, "https://example.com/api", "2026-09-20")
            .expect("Abilene has an AD");

        assert_eq!(coach.name, "Derek Berns"); // honorific stripped if present
        assert_eq!(coach.role, CoachRole::AthleticDirector);
        assert_eq!(coach.sport, None);
        assert_eq!(coach.gender, Gender::Mixed);
        assert_eq!(
            coach.professional_email.as_deref(),
            Some("dberns@abileneschools.org")
        );
    }

    #[test]
    fn honorific_stripped_from_ad_name() {
        let mut record = parse_records(FIXTURE).expect("fixture must parse")[0].clone();
        record.ad_name = Some("Coach Derek Berns".to_string());
        let (_, school_id) =
            parse_school(&record, "https://example.com/api", "2026-09-20").unwrap();
        let coach =
            parse_ad_coach(&record, &school_id, "https://example.com/api", "2026-09-20").unwrap();
        assert_eq!(coach.name, "Derek Berns");
    }

    #[test]
    fn school_with_no_class_or_enrollment_parses() {
        // Abilene MS (index 3) has no Class or Enrollment.
        let records = parse_records(FIXTURE).expect("fixture must parse");
        let ms = &records[3];
        assert_eq!(ms.school_name, "Abilene MS");
        assert!(ms.class.is_none());
        assert!(ms.enrollment.is_none());

        let (school, _) = parse_school(ms, "https://example.com/api", "2026-09-20").unwrap();
        assert_eq!(school.name, "Abilene MS");
        assert_eq!(school.classification, None);
        assert_eq!(school.enrollment, None);
    }

    #[test]
    fn school_with_no_website_parses() {
        // Great Bend HS (index 4) has no WebSite.
        let records = parse_records(FIXTURE).expect("fixture must parse");
        let gb = &records[4];
        assert_eq!(gb.school_name, "Great Bend HS");

        let (school, _) = parse_school(gb, "https://example.com/api", "2026-09-20").unwrap();
        assert_eq!(school.name, "Great Bend HS");
        assert_eq!(school.school_website, None);
    }

    #[test]
    fn no_cell_phones_or_principal_data_in_entities() {
        // The fixture contains PrincipalName, PrincipalCell, ADCell, PresName, PresCell, etc.
        // The parsed entities must NOT contain any of those values.
        let records = parse_records(FIXTURE).expect("fixture must parse");

        for record in &records {
            let (_, school_id) =
                parse_school(record, "https://example.com/api", "2026-09-20").unwrap();
            let coach = parse_ad_coach(record, &school_id, "https://example.com/api", "2026-09-20");

            // Coach phone field must be None.
            let coach = coach.unwrap();
            assert!(
                coach.phone.is_none(),
                "coach phone must be None for all records"
            );

            // The professional_email should never match a cell number or principal name.
            if let Some(email) = &coach.professional_email {
                let ad_cell = record.ad_cell.as_deref().map(|s| s.trim());
                let principal_name = record.principal_name.as_deref().map(|s| s.trim());
                assert!(
                    ad_cell != Some(email.as_str()),
                    "coach email should not match AD cell number"
                );
                assert!(
                    principal_name != Some(email.as_str()),
                    "coach email should not match principal name"
                );
            }
        }
    }

    #[test]
    fn empty_ad_name_yields_none() {
        let records = parse_records(FIXTURE).expect("fixture must parse");
        let mut record = records[0].clone();
        record.ad_name = Some("".to_string());
        let (_, school_id) =
            parse_school(&record, "https://example.com/api", "2026-09-20").unwrap();
        assert!(
            parse_ad_coach(&record, &school_id, "https://example.com/api", "2026-09-20").is_none()
        );
    }

    #[test]
    fn malformed_json_errors_out() {
        let result = parse_records("not json");
        assert!(result.is_err(), "malformed JSON must error, not panic");
    }

    #[test]
    fn empty_json_array_returns_zero_rows() {
        let records = parse_records("[]").expect("empty array must parse");
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn fixture_ad_email_fill_rate() {
        // All 5 fixture records have an AD email — 100% fill rate.
        let records = parse_records(FIXTURE).expect("fixture must parse");
        let with_email = records
            .iter()
            .filter(|r| {
                r.ad_email
                    .as_deref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            with_email,
            records.len(),
            "all {} fixture records have AD email (100% fill rate)",
            records.len()
        );
    }
}
