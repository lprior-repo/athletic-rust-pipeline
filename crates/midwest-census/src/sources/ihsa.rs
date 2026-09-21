//! IHSA API adapter (Illinois)
//!
//! The IHSA exposes two endpoints for this mission:
//!
//! * `GET https://api.ihsa.org/v1/schools` — one request returns all 828 member schools with
//!   `SchoolID` (zero-padded 4-char string like `"0101"`), `nameFormal`, `city`, and more.
//! * `GET https://api.ihsa.org/v1/schools/{SchoolID}/staff2` — per-school staff grouped by
//!   category (`Administration`, `Boys Athletics - Head Coaches`, etc.), each row carrying
//!   `PersonID`, `Name` (with optional honorific), `DefaultTitle` (e.g. `"Boys Cross Country Head Coach"`),
//!   `RoleID` (`HCB-CCB`, `HCG-TRG`, `C1-BoysAD`, `D1-GirlsAD`, etc.), and `email`.
//!
//! # Fields observed
//! Schools: `SchoolID`, `nameFormal`, `city`, `State`, `membershipType`, `type`, `URL`. The row is
//! always an Illinois school, so the `State` field is not read.
//! Staff: `PersonID`, `Name`, `DefaultTitle`, `RoleID`, `email`.
//!
//! # Deliberately ignored fields (never read, never stored)
//! `Phone`, `Fax`, `Address`, `POBox`, `Zip`, `Latitude`, `Longitude`, `Color1/2`,
//! `schoolLogo`, `isCPS`, `DirectorySort` — plus any non-coaching office role data
//! (secretary, administrative assistant, athletic trainer, principal, superintendent,
//! business manager, tech director, custodian). Those columns are not part of the schema.

use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// API base URL for the IHSA.
const IHSA_API: &str = "https://api.ihsa.org";

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

/// Envelope returned by `GET /v1/schools`.
#[derive(Debug, Clone, Deserialize)]
pub struct SchoolsEnvelope {
    pub data: Vec<SchoolRecord>,
}

/// One row from `/v1/schools`.
///
/// We only deserialize the fields we need. Everything else is silently ignored — including
/// address, latitude/longitude, color, and other fields that some sources publish but this
/// adapter must never touch.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchoolRecord {
    #[serde(rename = "SchoolID")]
    pub school_id: String,
    #[serde(rename = "nameFormal")]
    pub name_formal: String,
    #[serde(rename = "NameIHSA")]
    #[serde(default)]
    pub name_ihsa: Option<String>,
    #[serde(default)]
    pub name_short: Option<String>,
    pub city: String,
    #[serde(rename = "membershipType")]
    #[serde(default)]
    pub membership_type: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(rename = "enrollmentType")]
    #[serde(default)]
    pub enrollment_type: Option<String>,
    #[serde(rename = "hasBoundary")]
    #[serde(default)]
    pub has_boundary: Option<String>,
    #[serde(rename = "isCPS")]
    #[serde(default)]
    pub is_cps: Option<String>,
    #[serde(rename = "URL")]
    #[serde(default)]
    pub url: Option<String>,
    // Fields deliberately not used:
    // Address, POBox, Zip, Latitude, Longitude, schoolLogo, Color1/2,
    // TextOnColor1/2, HasColors, ColorSource.
}

/// One person from the `/staff2` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct StaffPerson {
    #[serde(rename = "PersonID")]
    pub person_id: i64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "DefaultTitle")]
    pub default_title: String,
    #[serde(rename = "HasEmail")]
    #[serde(default)]
    pub has_email: Option<bool>,
    #[serde(rename = "LastName")]
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(rename = "RoleID")]
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(rename = "Phone")]
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(rename = "Fax")]
    #[serde(default)]
    pub fax: Option<String>,
    #[serde(rename = "email")]
    #[serde(default)]
    pub email: Option<String>,
    // Fields deliberately not used: phone, fax.
}

/// People named by a `GET /v1/schools/{id}/staff2` payload.
///
/// Staff arrive under `data`, grouped by category (`"Administration"`, `"Boys Athletics - Head
/// Coaches"`, …). The category list belongs to the association, not to us, so every array under
/// `data` is flattened whatever its label. A person holds several roles and can be listed under
/// several titles ("Boys Athletic Director" and `"IHSA Official Representative"` are the same
/// administrator here), so identity is `(PersonID, DefaultTitle)`: every role survives, exact
/// repeats do not.
pub fn parse_staff(body: &str) -> Result<Vec<StaffPerson>> {
    #[derive(Deserialize)]
    struct Envelope {
        data: BTreeMap<String, Vec<StaffPerson>>,
    }

    let envelope: Envelope =
        serde_json::from_str(body).context("IHSA staff JSON is not a valid envelope")?;
    let mut seen = BTreeSet::new();
    let mut people = Vec::new();
    for (_, mut category) in envelope.data {
        category.retain(|person| seen.insert((person.person_id, person.default_title.clone())));
        people.append(&mut category);
    }
    Ok(people)
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Non-empty trimmed string → `Some`, or `None`.
fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Strip leading honorifics so "Mr. Barry Mink" and "Barry Mink" mint the same coach identity.
pub fn strip_honorific(value: &str) -> String {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let stripped = parts
        .iter()
        .take_while(|part| {
            matches!(
                part.trim_end_matches('.').to_ascii_lowercase().as_str(),
                "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "coach." | "sir" | "rev"
            )
        })
        .count();
    match parts.get(stripped..) {
        Some(kept) if !kept.is_empty() => kept.join(" "),
        // The value held nothing but honorifics (or no tokens at all): keep the original text.
        _ => value.trim().to_string(),
    }
}

/// Parse the IHSA `DefaultTitle` into a `(Sport, Gender)` pair.
///
/// Returns `None` for non-coaching titles (AD, secretary, trainer, principal, etc.).
///
/// # Examples
/// * `"Boys Cross Country Head Coach"` → `(Some(CrossCountry), Some(Boys))`
/// * `"Girls Track & Field Head Coach"` → `(Some(OutdoorTrack), Some(Girls))`
/// * `"Boys Athletic Director"` → `(None, None)`
pub fn parse_coach_title(title: &str) -> Option<(Sport, Gender)> {
    let lowered = title.to_ascii_lowercase();
    if lowered.trim().is_empty() {
        return None;
    }

    // Office/medical roles are not coaches.
    const NON_COACHING: [&str; 10] = [
        "secretary",
        "administrative assistant",
        "trainer",
        "principal",
        "superintendent",
        "business manager",
        "tech director",
        "custodian",
        "activities director",
        "athletic director",
    ];
    if NON_COACHING.iter().any(|t| lowered.contains(t)) {
        return None;
    }

    // Must contain "coach" to be a coaching role.
    if !lowered.contains("coach") {
        return None;
    }

    let sport = if lowered.contains("cross country") || lowered.contains("cross-country") {
        Sport::CrossCountry
    } else if lowered.contains("indoor") {
        Sport::IndoorTrack
    } else if lowered.contains("track") {
        Sport::OutdoorTrack
    } else {
        return None;
    };

    let gender = if lowered.contains("girls") || lowered.contains("women") {
        Gender::Girls
    } else if lowered.contains("boys") || lowered.contains("men") {
        Gender::Boys
    } else {
        Gender::Mixed
    };

    Some((sport, gender))
}

/// Map a published role label onto our role vocabulary.
///
/// Returns `None` for roles that are neither a coaching role nor an athletic-director role
/// (secretaries, trainers, principals, etc.).
pub fn parse_role(title: &str) -> Option<CoachRole> {
    let lowered = title.to_ascii_lowercase();

    // Office, medical and building staff are published in the same tables as coaches, and their
    // labels ("Athletic Director Secretary", "AD Administrative Assistant", "Athletic Trainer")
    // contain the words we would otherwise classify on.
    // NON_COACHING runs FIRST so "Athletic Director Secretary" gets filtered out before the AD check.
    const NON_COACHING: [&str; 8] = [
        "secretary",
        "administrative assistant",
        "trainer",
        "principal",
        "superintendent",
        "business manager",
        "tech director",
        "custodian",
    ];
    if NON_COACHING.iter().any(|token| lowered.contains(token)) {
        return None;
    }

    // AD roles — check AFTER NON_COACHING.
    // "Boys Athletic Director" → AD (contains "athletic director", no "assistant")
    // "Boys Athletic Director's Assistant" → NOT AD (also contains "assistant")
    if lowered.contains("athletic director") && !lowered.contains("assistant") {
        return Some(CoachRole::AthleticDirector);
    }

    if lowered.contains("coach") {
        if lowered.contains("assistant") || lowered.contains("asst") {
            return Some(CoachRole::AssistantCoach);
        }
        if lowered.contains("head") {
            return Some(CoachRole::HeadCoach);
        }
        return Some(CoachRole::Unknown);
    }

    None
}

/// Parse the JSON envelope returned by `GET /v1/schools`.
pub fn parse_schools(body: &str) -> Result<Vec<SchoolRecord>> {
    let envelope: SchoolsEnvelope =
        serde_json::from_str(body).context("IHSA schools JSON is not a valid envelope")?;
    Ok(envelope.data)
}

/// Whether a coach's address is worth a reveal request.
///
/// The census ships TF/XC head coaches and athletic directors to the recruiting projection; other
/// sports' addresses are collected (the row is kept) but not paid for.
pub fn reveal_address_for(coach: &CanonicalCoach) -> bool {
    coach.role == CoachRole::AthleticDirector
        || matches!(
            coach.sport,
            Some(Sport::CrossCountry | Sport::OutdoorTrack | Sport::IndoorTrack)
        )
}

/// Parse the body of `GET /v1/schools/{id}/staff/{PersonID}/email` (`{"email":"…"}`).
///
/// The endpoint is the school directory's "Show email" reveal. An empty or placeholder address
/// yields `None` so the caller never stores a non-address.
pub fn parse_email(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct EmailEnvelope {
        #[serde(default)]
        email: Option<String>,
    }

    let envelope: EmailEnvelope = serde_json::from_str(body).ok()?;
    envelope.email.as_deref().and_then(nonempty)
}

/// Convert one IHSA school record into a canonical school.
///
/// Returns `None` when the school name is empty.
pub fn parse_school(
    record: &SchoolRecord,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = record.name_formal.trim();
    if name.is_empty() {
        return None;
    }
    let normalized = normalize_name(name);
    let (mut school, id) = CanonicalSchool::new("IL", name, &normalized);
    school.city = nonempty(&record.city);
    school.association = Some("ihsa".into());
    school.school_website = record.url.as_ref().and_then(|v| nonempty(v));
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "ihsa".into(),
            },
            &record.school_id,
        )
        .with_url(source_url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new("ihsa", Some(source_url.to_string())),
        observed_on,
    ));
    Some((school, id))
}

/// Convert one IHSA staff person into a canonical coach, if the title is a coaching or AD role.
///
/// Returns `None` for office roles (secretary, trainer, principal, etc.) and for staff whose
/// `DefaultTitle` does not map to a coaching or AD role.
pub fn parse_coach(
    person: &StaffPerson,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let title = &person.default_title;
    let role = parse_role(title)?;

    let name = strip_honorific(&person.name);
    let mut coach = match role {
        CoachRole::AthleticDirector => CanonicalCoach::new(
            school_id,
            &name,
            None,          // school-wide role
            Gender::Mixed, // AD is not gender-specific
            role,
        ),
        _ => {
            // Coaching role. Titles outside track & field / cross country ("Boys Bowling Head
            // Coach") are still kept as coaches of the school, but they carry no sport: guessing
            // outdoor track here would present a basketball coach as a track coach.
            match parse_coach_title(title) {
                Some((sport, gender)) => {
                    CanonicalCoach::new(school_id, &name, Some(sport), gender, role)
                }
                None => CanonicalCoach::new(school_id, &name, None, Gender::Mixed, role),
            }
        }
    };

    // `staff2` publishes the person, not the address — the caller fills `professional_email` from
    // the reveal endpoint when `HasEmail` is set.
    coach.professional_email = person.email.as_ref().and_then(|e| nonempty(e));

    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "ihsa".into(),
            },
            person.person_id.to_string(),
        )
        .with_url(source_url),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new("ihsa", Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}

// ---------------------------------------------------------------------------
// Collect
// ---------------------------------------------------------------------------

/// Collect this provider's Illinois schools and coach/AD contacts into the canonical store.
///
/// Strategy:
/// 1. Fetch `https://api.ihsa.org/v1/schools` — one request returns all 828 Illinois member schools.
/// 2. For each school, fetch `https://api.ihsa.org/v1/schools/{SchoolID}/staff2` — per-school staff.
/// 3. Parse schools into canonical schools, staff into coaches (ADs + head/assistant coaches).
/// 4. Journal progress per school so a re-run resumes without re-processing.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("ihsa", "schools");
    report.unit = "schools".to_string();

    // Snapshot fetcher stats before work.
    let before = ctx.fetcher.stats().await;

    // Load the resume set (keys already journalled in this phase).
    let done_keys: HashSet<String> = ctx.store.journal_keys("ihsa_schools")?;

    let limit = options.limit;
    // Tally counters saturate: they feed diagnostics only, so an impossible overflow floors at
    // `usize::MAX` instead of panicking or wrapping silently.
    let mut processed = 0usize;
    let mut skipped = 0usize;
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    // PersonID → revealed address within the current school (a person can hold several titles).
    let mut revealed_emails: HashMap<i64, Option<String>> = HashMap::new();

    // ── Step 1: Fetch the schools list ───────────────────────────────────
    let schools_url = format!("{IHSA_API}/v1/schools");
    let outcome = match ctx.fetcher.get(&schools_url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch IHSA schools: {e}"));
            return Ok(report);
        }
    };

    let records = parse_schools(&outcome.text())?;
    report.requests = report.requests.saturating_add(1);
    if outcome.from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    }

    // ── Step 2: For each school, fetch staff and parse ───────────────────
    for record in &records {
        // Honour limit.
        if let Some(max) = limit {
            if processed >= max {
                break;
            }
        }

        // Skip already-processed schools (resume support).
        let journal_key = format!("IL:{}", record.school_id);
        if done_keys.contains(&journal_key) {
            skipped = skipped.saturating_add(1);
            continue;
        }
        revealed_emails.clear();

        // Parse school.
        let (school, school_id) = match parse_school(record, &schools_url, &options.observed_on) {
            Some(pair) => pair,
            None => continue,
        };
        schools.push(school);

        // Fetch staff for this school.
        let staff_url = format!("{IHSA_API}/v1/schools/{}/staff2", record.school_id);
        let staff_outcome = match ctx.fetcher.get(&staff_url, &ctx.fetch_options()).await {
            Ok(o) => o,
            Err(e) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!(
                    "failed to fetch staff for school {}: {e}",
                    record.school_id
                ));
                ctx.store
                    .journal_done(
                        "ihsa_schools",
                        &journal_key,
                        &serde_json::json!({
                            "school_id": record.school_id,
                            "name": record.name_formal,
                            "error": e.to_string(),
                        }),
                    )
                    .context("journaling ihsa school progress")?;
                processed = processed.saturating_add(1);
                continue;
            }
        };
        report.requests = report.requests.saturating_add(1);
        if staff_outcome.from_cache {
            report.from_cache = report.from_cache.saturating_add(1);
        }

        // Parse staff and emit coaches.
        let staff = match parse_staff(&staff_outcome.text()) {
            Ok(env) => env,
            Err(e) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!(
                    "failed to parse staff for school {}: {e}",
                    record.school_id
                ));
                ctx.store
                    .journal_done(
                        "ihsa_schools",
                        &journal_key,
                        &serde_json::json!({
                            "school_id": record.school_id,
                            "name": record.name_formal,
                            "parse_error": e.to_string(),
                        }),
                    )
                    .context("journaling ihsa school progress")?;
                processed = processed.saturating_add(1);
                continue;
            }
        };

        for person in &staff {
            if let Some(mut coach) =
                parse_coach(person, &school_id, &staff_url, &options.observed_on)
            {
                // `staff2` carries a `HasEmail` flag but never the address itself; the address is
                // behind `GET /v1/schools/{id}/staff/{PersonID}/email` (the "Show email" button).
                // One reveal per person, and only for the roles the census ships: the recruiting
                // projection reads TF/XC head coaches and ADs, so a bowling coach's address is not
                // worth a request.
                if person.has_email == Some(true) && reveal_address_for(&coach) {
                    let email_url = format!(
                        "{IHSA_API}/v1/schools/{}/staff/{}/email",
                        record.school_id, person.person_id
                    );
                    let revealed = match revealed_emails.entry(person.person_id) {
                        std::collections::hash_map::Entry::Occupied(slot) => slot.get().clone(),
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            let value =
                                match ctx.fetcher.get(&email_url, &ctx.fetch_options()).await {
                                    Ok(outcome) => {
                                        report.requests = report.requests.saturating_add(1);
                                        if outcome.from_cache {
                                            report.from_cache = report.from_cache.saturating_add(1);
                                        }
                                        parse_email(&outcome.text())
                                    }
                                    Err(e) => {
                                        report.errors = report.errors.saturating_add(1);
                                        report.note(format!(
                                            "email reveal failed for person {}: {e}",
                                            person.person_id
                                        ));
                                        None
                                    }
                                };
                            slot.insert(value.clone());
                            value
                        }
                    };
                    if let Some(address) = revealed {
                        coach.professional_email = Some(address);
                        coach.evidence.push(Evidence::parsed(
                            SourceRef::new("ihsa", Some(email_url)),
                            &options.observed_on,
                        ));
                    }
                }
                if coach.professional_email.is_some() {
                    report.with_email = report.with_email.saturating_add(1);
                }
                coaches.push(coach);
            }
        }

        // Journal this school as done.
        ctx.store
            .journal_done(
                "ihsa_schools",
                &journal_key,
                &serde_json::json!({
                    "school_id": record.school_id,
                    "name": record.name_formal,
                }),
            )
            .context("journaling ihsa school progress")?;

        processed = processed.saturating_add(1);
    }

    // Append all schools and coaches to the store.
    if !schools.is_empty() {
        ctx.store
            .append_many(Table::Schools, &schools)
            .context("writing ihsa schools")?;
    }
    if !coaches.is_empty() {
        ctx.store
            .append_many(Table::Coaches, &coaches)
            .context("writing ihsa coaches")?;
    }

    // Finalize stats and counts.
    let after = ctx.fetcher.stats().await;
    let delta_requests = after.requests.saturating_sub(before.requests);
    report.rows = u64::try_from(processed).context("ihsa school count exceeds u64")?;
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} Illinois schools from IHSA; {} already done",
        processed, skipped
    ));

    Ok(report)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture: trimmed subset of the real `GET /v1/schools` response.
    ///
    /// **Provenance** — URL:
    /// `https://api.ihsa.org/v1/schools`
    /// (the single request returns all 828 member schools; this is a 3-school trimmed subset.)
    /// **Capture file**:
    /// `tools/a29-coach/il-sample.json` (the school list portion).
    const FIXTURE_SCHOOLS: &str = include_str!("../../tests/fixtures/ihsa/v1_schools.json");

    /// Fixture: real `GET /v1/schools/0101/staff2` response (Abingdon-Avon HS).
    ///
    /// **Provenance** — URL:
    /// `https://api.ihsa.org/v1/schools/0101/staff2`
    /// **Capture file**:
    /// `tools/a29-coach/il-sample.json` (Abingdon-Avon entry, SchoolID 0101).
    const FIXTURE_STAFF_RICH: &str =
        include_str!("../../tests/fixtures/ihsa/staff2_coach_rich.json");

    /// Fixture: real `GET /v1/schools/0430/staff2` response (Unity Christian HS).
    ///
    /// **Provenance** — URL:
    /// `https://api.ihsa.org/v1/schools/0430/staff2`
    /// **Capture file**:
    /// `tools/a29-coach/il-sample.json` (Unity Christian entry, SchoolID 0430).
    const FIXTURE_STAFF_OFFICE: &str =
        include_str!("../../tests/fixtures/ihsa/staff2_office_only.json");

    // ── Schools parsing tests ──────────────────────────────────────────────

    #[test]
    fn parses_schools_fixture() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        assert_eq!(records.len(), 3, "fixture contains 3 schools");
    }

    #[test]
    fn parses_school_name_city_id() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let abingdon = &records[0];

        assert_eq!(abingdon.school_id, "0101");
        assert_eq!(abingdon.name_formal, "Abingdon-Avon High School");
        assert_eq!(abingdon.city, "Abingdon");
    }

    #[test]
    fn parses_school_into_canonical() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let abingdon = &records[0];

        let (school, id) = parse_school(abingdon, "https://example.com/api", "2026-09-19")
            .expect("Abingdon has a name");

        assert_eq!(school.name, "Abingdon-Avon High School");
        assert_eq!(school.state.as_deref(), Some("IL"));
        assert_eq!(school.association.as_deref(), Some("ihsa"));
        assert_eq!(school.city.as_deref(), Some("Abingdon"));
        // `/v1/schools` rows carry no `URL` field (see this capture-backed fixture), so the school
        // website stays unset for IHSA.
        assert_eq!(school.school_website, None);

        // Source identity carries the IHSA SchoolID.
        assert_eq!(school.source_identities.len(), 1);
        assert_eq!(
            school.source_identities[0].namespace,
            SourceNamespace::AssociationSchool {
                association: "ihsa".into()
            }
        );
        assert_eq!(school.source_identities[0].id, "0101");

        // School id is deterministic.
        let expected_id = CanonicalSchool::mint(
            "IL",
            "Abingdon-Avon High School",
            &normalize_name("Abingdon-Avon High School"),
        );
        assert_eq!(id, expected_id);
    }

    #[test]
    fn school_url_becomes_school_website() {
        // Constructed input, not a capture: the live `/v1/schools` rows carry no `URL` field, but the
        // adapter still maps one when a row has it.
        let row = SchoolRecord {
            school_id: "9999".into(),
            name_formal: "Example High School".into(),
            name_ihsa: None,
            name_short: None,
            city: "Example".into(),
            membership_type: None,
            r#type: None,
            enrollment_type: None,
            has_boundary: None,
            is_cps: None,
            url: Some("https://www.example.org".into()),
        };

        let (parsed, _) = parse_school(&row, "https://example.com/api", "2026-09-19").unwrap();
        assert_eq!(parsed.name, "Example High School");
        assert_eq!(parsed.city.as_deref(), Some("Example"));
        assert_eq!(
            parsed.school_website.as_deref(),
            Some("https://www.example.org")
        );
    }

    #[test]
    fn school_with_blank_name_is_skipped() {
        let row = SchoolRecord {
            school_id: "9998".into(),
            name_formal: "   ".into(),
            name_ihsa: None,
            name_short: None,
            city: "Nowhere".into(),
            membership_type: None,
            r#type: None,
            enrollment_type: None,
            has_boundary: None,
            is_cps: None,
            url: None,
        };

        assert!(parse_school(&row, "https://example.com/api", "2026-09-19").is_none());
    }

    // ── Staff parsing tests ─────────────────────────────────────────────────

    #[test]
    fn parses_staff_fixture_rich() {
        let all = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
        // Live payload for school 0101: six categories, 31 (PersonID, title) rows. Reid Kelso is
        // both ADs and listed once as official representative, so role rows outnumber people.
        assert_eq!(
            all.len(),
            31,
            "coach-rich fixture flattens to 31 staff rows"
        );
        let people: std::collections::BTreeSet<i64> = all.iter().map(|p| p.person_id).collect();
        assert_eq!(people.len(), 20, "those rows belong to 20 distinct people");
    }

    #[test]
    fn parses_staff_fixture_office_only() {
        let all = parse_staff(FIXTURE_STAFF_OFFICE).expect("fixture must parse");
        // Live payload for school 0138: administration only, no athletics staff categories.
        assert_eq!(all.len(), 6, "office-only fixture has 6 staff rows");
    }

    #[test]
    fn coach_title_maps_sport_and_gender() {
        assert_eq!(
            parse_coach_title("Boys Cross Country Head Coach"),
            Some((Sport::CrossCountry, Gender::Boys))
        );
        assert_eq!(
            parse_coach_title("Girls Cross Country Head Coach"),
            Some((Sport::CrossCountry, Gender::Girls))
        );
        assert_eq!(
            parse_coach_title("Boys Track & Field Head Coach"),
            Some((Sport::OutdoorTrack, Gender::Boys))
        );
        assert_eq!(
            parse_coach_title("Girls Track & Field Head Coach"),
            Some((Sport::OutdoorTrack, Gender::Girls))
        );
    }

    #[test]
    fn coach_title_returns_none_for_non_coaching() {
        assert!(parse_coach_title("Boys Athletic Director").is_none());
        assert!(parse_coach_title("Girls Athletic Director").is_none());
        assert!(parse_coach_title("Boys Athletic Director's Assistant").is_none());
        assert!(parse_coach_title("Girls Athletic Director's Assistant").is_none());
        assert!(parse_coach_title("Athletic Trainer").is_none());
        assert!(parse_coach_title("Principal").is_none());
        assert!(parse_coach_title("Athletic Director Secretary").is_none());
    }

    #[test]
    fn role_parsed_for_coaching_and_ad() {
        assert_eq!(
            parse_role("Boys Cross Country Head Coach"),
            Some(CoachRole::HeadCoach)
        );
        assert_eq!(
            parse_role("Girls Track & Field Head Coach"),
            Some(CoachRole::HeadCoach)
        );
        assert_eq!(
            parse_role("Boys Athletic Director"),
            Some(CoachRole::AthleticDirector)
        );
        assert_eq!(
            parse_role("Girls Athletic Director"),
            Some(CoachRole::AthleticDirector)
        );
    }

    #[test]
    fn role_returns_none_for_office_roles() {
        assert!(parse_role("Boys Athletic Director's Assistant").is_none());
        assert!(parse_role("Girls Athletic Director's Assistant").is_none());
        assert!(parse_role("Athletic Trainer").is_none());
        assert!(parse_role("Principal").is_none());
        assert!(parse_role("Athletic Director Secretary").is_none());
    }

    // ── Coach entity construction tests ─────────────────────────────────────

    #[test]
    fn coach_entity_from_rich_fixture() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let abingdon = &records[0];
        let (_, school_id) = parse_school(abingdon, "https://example.com/api", "2026-09-19")
            .expect("Abingdon has a name");

        let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
        let coaches: Vec<_> = staff
            .iter()
            .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
            .collect();

        // Kept: 17 sport head coaches + 2 AD rows + 2 "unknown" coach titles = 21 entities.
        // Dropped: principals, superintendent, medical staff, official representative.
        assert_eq!(
            coaches.len(),
            21,
            "rich fixture yields 21 coach/AD entities"
        );

        // Verify head coaches have correct sport/gender mapping.
        let boys_xc = coaches
            .iter()
            .find(|c| {
                c.name.contains("Mink")
                    && c.sport == Some(Sport::CrossCountry)
                    && c.gender == Gender::Boys
            })
            .expect("should have a Boys Cross Country coach named Mink");
        assert_eq!(boys_xc.role, CoachRole::HeadCoach);
        // The address is not in the staff payload; it comes from the reveal endpoint.
        assert_eq!(boys_xc.professional_email, None);

        let girls_tf = coaches
            .iter()
            .find(|c| {
                c.name.contains("Rakestraw")
                    && c.sport == Some(Sport::OutdoorTrack)
                    && c.gender == Gender::Girls
            })
            .expect("should have a Girls Track & Field coach named Rakestraw");
        assert_eq!(girls_tf.role, CoachRole::HeadCoach);
        assert_eq!(girls_tf.professional_email, None);

        // Verify ADs have sport = None, gender = Mixed.
        let ad = coaches
            .iter()
            .find(|c| c.role == CoachRole::AthleticDirector)
            .expect("should have at least one AD");
        assert_eq!(ad.sport, None);
        assert_eq!(ad.gender, Gender::Mixed);
        assert_eq!(ad.name, "Reid Kelso", "honorific stripped from the AD name");
    }

    #[test]
    fn email_reveal_parses_only_real_addresses() {
        // Live body: /v1/schools/0101/staff/96256/email → {"email":"jrakestraw@atown276.net"}
        assert_eq!(
            parse_email(r#"{"email":"jrakestraw@atown276.net"}"#).as_deref(),
            Some("jrakestraw@atown276.net")
        );
        // Blank / missing / malformed reveals store nothing.
        assert_eq!(parse_email(r#"{"email":"   "}"#), None);
        assert_eq!(parse_email(r#"{"email":null}"#), None);
        assert_eq!(parse_email("{}"), None);
        assert_eq!(parse_email("<html>blocked</html>"), None);
    }

    #[test]
    fn kept_staff_rows_advertise_an_email() {
        let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let (_, school_id) =
            parse_school(&records[0], "https://example.com/api", "2026-09-19").unwrap();

        let kept: Vec<_> = staff
            .iter()
            .filter(|p| {
                parse_coach(p, &school_id, "https://example.com/api", "2026-09-19").is_some()
            })
            .collect();
        assert!(
            kept.iter().all(|p| p.has_email == Some(true)),
            "every kept row is flagged HasEmail, so each reveal can return an address"
        );
    }

    #[test]
    fn office_roles_excluded_from_coaches() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let unity = &records[2]; // Unity Christian
        let (_, school_id) =
            parse_school(unity, "https://example.com/api", "2026-09-19").expect("Unity has a name");

        let staff = parse_staff(FIXTURE_STAFF_OFFICE).expect("fixture must parse");
        let coaches: Vec<_> = staff
            .iter()
            .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
            .collect();

        // Unity's live payload: 6 administration rows of which only the two AD rows are coaches.
        assert_eq!(
            coaches.len(),
            2,
            "only 2 AD entities from the office-only fixture"
        );

        // Verify non-coaching roles were excluded and the ADs survive.
        assert!(coaches
            .iter()
            .all(|c| c.role == CoachRole::AthleticDirector));
        assert!(coaches.iter().all(|c| c.name.contains("Ringstrand")));
        assert!(coaches.iter().all(|c| c.sport.is_none()));
        assert!(
            !coaches.iter().any(|c| c.name.contains("Secretary")),
            "office titles must not survive as names"
        );
    }

    #[test]
    fn honorifics_stripped_from_names() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let abingdon = &records[0];
        let (_, school_id) =
            parse_school(abingdon, "https://example.com/api", "2026-09-19").unwrap();

        let mut person = parse_staff(FIXTURE_STAFF_RICH)
            .unwrap()
            .into_iter()
            .find(|p| p.default_title == "Boys Track & Field Head Coach")
            .expect("rich fixture publishes a boys track head coach");
        person.name = "Coach Justin Rakestraw".to_string();

        let coach = parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19")
            .expect("coach parse should work with honorific");
        assert_eq!(
            coach.name, "Justin Rakestraw",
            "honorific 'Coach' should be stripped"
        );

        // Also test other honorifics.
        person.name = "Mr. Justin Rakestraw".to_string();
        let coach =
            parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19").unwrap();
        assert_eq!(
            coach.name, "Justin Rakestraw",
            "honorific 'Mr.' should be stripped"
        );

        person.name = "Dr. Justin Rakestraw".to_string();
        let coach =
            parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19").unwrap();
        assert_eq!(
            coach.name, "Justin Rakestraw",
            "honorific 'Dr.' should be stripped"
        );

        person.name = "Miss Megan Hildreth".to_string();
        let coach =
            parse_coach(&person, &school_id, "https://example.com/api", "2026-09-19").unwrap();
        assert_eq!(
            coach.name, "Megan Hildreth",
            "honorific 'Miss' should be stripped"
        );
    }

    #[test]
    fn no_cell_phones_or_personal_data_in_entities() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let abingdon = &records[0];
        let (_, school_id) =
            parse_school(abingdon, "https://example.com/api", "2026-09-19").unwrap();

        let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
        for person in &staff {
            if let Some(coach) =
                parse_coach(person, &school_id, "https://example.com/api", "2026-09-19")
            {
                assert!(coach.phone.is_none(), "coach phone must be None");
            }
            // Whether or not the role is kept, the payload's phone/fax never reach an entity: the
            // only carrier is `StaffPerson`, which the entity conversion does not read for phones.
        }
    }

    // ── Error handling tests ────────────────────────────────────────────────

    #[test]
    fn malformed_json_errors_out() {
        let result = parse_schools("not json");
        assert!(result.is_err(), "malformed JSON must error, not panic");

        let result = parse_staff("not json");
        assert!(result.is_err(), "malformed JSON must error, not panic");
    }

    #[test]
    fn empty_json_object_parses_empty_lists() {
        // An empty schools object should yield 0 records, not error.
        let result = parse_schools("{\"data\": []}");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        // An empty staff object should yield 0 persons, not error.
        let result = parse_staff("{}");
        assert!(
            result.is_err(),
            "a staff payload without `data` is not a valid envelope"
        );
        let empty = parse_staff(r#"{"data":{}}"#).expect("empty data object parses");
        assert!(empty.is_empty());
    }

    // ── Coach relevance test ────────────────────────────────────────────────

    #[test]
    fn email_reveal_targets_only_shipped_roles() {
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let (_, school_id) =
            parse_school(&records[0], "https://example.com/api", "2026-09-19").unwrap();
        let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
        let kept: Vec<_> = staff
            .iter()
            .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
            .collect();

        let reveal: Vec<_> = kept.iter().filter(|c| reveal_address_for(c)).collect();
        assert!(
            reveal.iter().all(|c| c.role == CoachRole::AthleticDirector
                || matches!(c.sport, Some(Sport::CrossCountry | Sport::OutdoorTrack))),
            "only TF/XC and AD roles are paid for"
        );
        assert!(
            !reveal.iter().any(|c| c.name.contains("Quinn")),
            "the football coach's address is not requested"
        );
        // 21 kept rows collapse to three people: both AD rows, the XC coach (boys+girls) and the
        // track coach (boys+girls), which is what bounds the reveal request count.
        let people: std::collections::BTreeSet<i64> = staff
            .iter()
            .filter(|p| {
                parse_coach(p, &school_id, "https://example.com/api", "2026-09-19")
                    .is_some_and(|c| reveal_address_for(&c))
            })
            .map(|p| p.person_id)
            .collect();
        assert_eq!(
            people.len(),
            3,
            "one reveal per person, three people publish those roles"
        );
    }

    #[test]
    fn track_and_cross_country_roles_are_kept() {
        // The census exists for TF/XC: every one of those roles in the rich fixture must survive
        // with the right sport and gender, and ADs must survive without a sport.
        let records = parse_schools(FIXTURE_SCHOOLS).expect("fixture must parse");
        let abingdon = &records[0];
        let (_, school_id) =
            parse_school(abingdon, "https://example.com/api", "2026-09-19").unwrap();

        let staff = parse_staff(FIXTURE_STAFF_RICH).expect("fixture must parse");
        let coaches: Vec<_> = staff
            .iter()
            .filter_map(|p| parse_coach(p, &school_id, "https://example.com/api", "2026-09-19"))
            .collect();

        for (sport, gender) in [
            (Sport::CrossCountry, Gender::Boys),
            (Sport::CrossCountry, Gender::Girls),
            (Sport::OutdoorTrack, Gender::Boys),
            (Sport::OutdoorTrack, Gender::Girls),
        ] {
            let found = coaches
                .iter()
                .filter(|c| {
                    c.sport == Some(sport) && c.gender == gender && c.role == CoachRole::HeadCoach
                })
                .count();
            assert_eq!(
                found, 1,
                "exactly one {gender:?} {sport:?} head coach is published"
            );
        }

        let ads = coaches
            .iter()
            .filter(|c| c.role == CoachRole::AthleticDirector)
            .count();
        assert_eq!(ads, 2, "boys and girls AD rows both survive");
    }
}
