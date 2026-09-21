//! Import the researched official coach-contact dataset into canonical entities.
//!
//! The dataset (`data/coach-contacts.csv` in the research workspace) is the consolidated output of the
//! official-source contact graph: one row per (school, sport, role) with the professional email that
//! the school or state association published for that role. Importing it is an *artifact import*, not
//! a crawl: the rows already carry `source_url` + `last_observed`, so each canonical entity can cite
//! the exact page it came from.
//!
//! What is deliberately not read, even when the upstream capture contained it: home phone numbers,
//! home addresses, cell numbers, athlete contacts. Those columns are not part of this schema, so they
//! cannot leak through the importer.

use crate::sources::AdapterReport;
use crate::store::{Store, Table};
use anyhow::{Context, Result};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachId, CoachRole, Evidence, Gender,
    SchoolId, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::path::Path;

/// One row of the contact dataset. Field names are the published CSV header.
#[derive(Debug, Clone, Deserialize)]
pub struct CoachContactRow {
    pub school: String,
    #[serde(default)]
    pub city: String,
    pub state: String,
    #[serde(default)]
    pub sport: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub coach_name: String,
    #[serde(default)]
    pub public_professional_email: String,
    #[serde(default)]
    pub ad_name: String,
    #[serde(default)]
    pub ad_email: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub last_observed: String,
}

/// A school plus the coaches derived from one CSV row.
#[derive(Debug, Clone)]
pub struct RowEntities {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
}

fn clean(value: &str) -> String {
    value.trim().to_string()
}

/// Strip leading honorifics so "Mr. Barry Mink" and "Barry Mink" mint the same coach identity.
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

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Map a published sport label onto our ontology plus the gender side it covers.
///
/// Examples that must work: `Boys Track and Field`, `Varsity Head Coach - Girls Cross Country`,
/// `Boys Cross Country Head Coach`, `Girls Track & Field Head Coach`.
pub fn parse_sport(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.to_ascii_lowercase();
    if lowered.trim().is_empty() {
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

/// Map a published role label onto our role vocabulary. Returns `None` for roles that are neither a
/// coaching role nor an athletic-director role (secretaries, trainers, principals), which keeps the
/// coach table free of non-coaching staff.
pub fn parse_role(label: &str) -> Option<CoachRole> {
    let lowered = label.to_ascii_lowercase();
    // Office, medical and building staff are published in the same tables as coaches, and their
    // labels ("Athletic Director Secretary", "AD Administrative Assistant", "Athletic Trainer")
    // contain the words we would otherwise classify on.
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
    if lowered.contains("athletic director") || lowered.contains("activities director") {
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

/// The association namespace a captured contact URL belongs to, so that the school identity carries
/// the origin's own key space rather than a bare name.
fn namespace_for_url(url: &str) -> SourceNamespace {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let association = if host.contains("wiaawi") {
        "wiaa"
    } else if host.contains("kshsaa") {
        "kshsaa"
    } else if host.contains("ihsa") {
        "ihsa"
    } else if host.contains("myohsaa") {
        "ohsaa"
    } else if host.contains("mshsl") {
        "mshsl"
    } else if host.contains("mhsaa") {
        "mhsaa"
    } else if host.contains("nsaahome") {
        "nsaa"
    } else if host.contains("ndhsaa") {
        "ndhsaa"
    } else if host.contains("gobound") {
        return SourceNamespace::Other("bound".to_string());
    } else {
        return SourceNamespace::Other("coach_contacts_csv".to_string());
    };
    SourceNamespace::AssociationSchool {
        association: association.to_string(),
    }
}

/// Provider key extracted from a captured URL when the provider uses one (WIAA `orgID`, IHSA
/// `/schools/<id>/…`). `None` means the provider is name-keyed, which is recorded as such.
fn identity_key(url: &str) -> Option<String> {
    let query_key = |name: &str| -> Option<String> {
        url.split(['?', '&'])
            .find_map(|part| part.strip_prefix(&format!("{name}=")))
            .map(|value| value.split(['&', '#']).next().unwrap_or(value).to_string())
    };
    if let Some(value) = query_key("orgID") {
        return Some(value);
    }
    if let Some(value) = query_key("s") {
        return Some(value);
    }
    let segments: Vec<&str> = url.split('/').collect();
    for (index, segment) in segments.iter().enumerate() {
        if *segment == "schools" || *segment == "school" {
            if let Some(candidate) = segments.get(index.saturating_add(1)) {
                let candidate = candidate.trim();
                if !candidate.is_empty() && candidate.chars().all(|ch| ch.is_ascii_digit()) {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

/// Build the canonical entities for one CSV row.
pub fn row_entities(row: &CoachContactRow, default_observed_on: &str) -> Result<RowEntities> {
    let state = clean(&row.state).to_ascii_uppercase();
    let school_name = clean(&row.school);
    let (mut school, school_id) = school_with_city(&state, &school_name, &row.city);
    let source = RowSource::of(row, default_observed_on, &school_name);
    school.source_identities.push(
        SourceIdentity::new(source.namespace.clone(), source.key.clone())
            .with_url(source.url.clone().unwrap_or_default()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new("coach_contacts_csv", source.url.clone()),
        source.observed_on.clone(),
    ));

    let role = parse_role(&format!("{} {}", row.role, row.sport));
    let mut coaches = Vec::new();

    if let Some(coach) = primary_coach(row, &school_id, &source, role)? {
        coaches.push(coach);
    }

    // Coaching rows in IA/IL/NE/OH/WI also carry the school's AD columns; import that person so the
    // contact graph has an athletic office even where no dedicated AD row was captured. Rows whose
    // role is a non-athletic office (Superintendent/Principal/Trainer/Secretary) are dropped above and
    // never reach here, so their names cannot leak through this branch.
    if matches!(
        role,
        Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown)
    ) {
        if let Some(coach) = imported_ad(row, &school_id, &source) {
            coaches.push(coach);
        }
    }

    Ok(RowEntities { school, coaches })
}

/// The source coordinates one CSV row carries: when it was observed, the provider namespace and
/// identity key, the row's URL, and the evidence reference every entity cites.
struct RowSource {
    observed_on: String,
    namespace: SourceNamespace,
    key: String,
    url: Option<String>,
    source_ref: SourceRef,
}

impl RowSource {
    /// Read one row's source coordinates; `school_name` keys the identity when the row has no URL.
    fn of(row: &CoachContactRow, default_observed_on: &str, school_name: &str) -> Self {
        let observed_on = if row.last_observed.trim().is_empty() {
            default_observed_on.to_string()
        } else {
            clean(&row.last_observed)
        };
        let url = nonempty(&row.source_url);
        let namespace = url
            .as_deref()
            .map(namespace_for_url)
            .unwrap_or_else(|| SourceNamespace::Other("coach_contacts_csv".to_string()));
        let key = url
            .as_deref()
            .and_then(identity_key)
            .unwrap_or_else(|| normalize_name(school_name));
        Self {
            source_ref: SourceRef::new("coach_contacts_csv", url.clone()),
            observed_on,
            namespace,
            key,
            url,
        }
    }
}

/// The canonical school one CSV row describes, with the city alias the row publishes.
fn school_with_city(state: &str, school_name: &str, city: &str) -> (CanonicalSchool, SchoolId) {
    let (mut school, school_id) =
        CanonicalSchool::new(state, school_name, normalize_name(school_name));
    school.city = nonempty(city);
    if let Some(city) = school.city.clone() {
        school.aliases.push(format!("{city} {state}"));
    }
    (school, school_id)
}

/// The sport a row's label resolves to (`None` when it names none) and the gender side it covers.
fn sport_of(row: &CoachContactRow) -> (Option<Sport>, Gender) {
    let sport_gender = parse_sport(&row.sport);
    let (sport, gender) = sport_gender.unwrap_or((Sport::OutdoorTrack, Gender::Mixed));
    (sport_gender.map(|_| sport), gender)
}

/// The role a sport-scoped coaching row must name; the message is the one the builder raised.
fn required_role(role: Option<CoachRole>) -> Result<CoachRole> {
    role.ok_or_else(|| anyhow::anyhow!("coaching role"))
}

/// Set a coach row's email and attach its provider identity and evidence.
fn attach_source(
    coach: &mut CanonicalCoach,
    identity: String,
    email: Option<String>,
    source: &RowSource,
) {
    coach.professional_email = email;
    coach.source_identities.push(
        SourceIdentity::new(source.namespace.clone(), identity)
            .with_url(source.url.clone().unwrap_or_default()),
    );
    coach.evidence.push(Evidence::parsed(
        source.source_ref.clone(),
        source.observed_on.clone(),
    ));
}

/// The primary coach entity one CSV row describes: a sport coach, a school-wide AD, or nothing.
fn primary_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
    role: Option<CoachRole>,
) -> Result<Option<CanonicalCoach>> {
    match role {
        // A sport-scoped coaching row.
        Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown) => {
            sport_coach(row, school_id, source, role)
        }
        // A school-wide athletic-director row.
        Some(CoachRole::AthleticDirector) => Ok(ad_coach(row, school_id, source)),
        None => Ok(None),
    }
}

/// The sport-scoped coaching entity one row describes, when the row names a coach.
fn sport_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
    role: Option<CoachRole>,
) -> Result<Option<CanonicalCoach>> {
    let Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown) = role else {
        return Ok(None);
    };
    let Some(coach_name) = nonempty(&row.coach_name) else {
        return Ok(None);
    };
    let (sport, gender) = sport_of(row);
    let identity = format!("{}:{role:?}", source.key);
    let email = nonempty(&row.public_professional_email);
    let mut coach = CanonicalCoach::new(
        school_id,
        strip_honorific(&coach_name),
        sport,
        gender,
        required_role(role)?,
    );
    attach_source(&mut coach, identity, email, source);
    Ok(Some(coach))
}

/// The school-wide athletic-director entity an AD row describes.
fn ad_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
) -> Option<CanonicalCoach> {
    let ad_name = nonempty(&row.ad_name).or_else(|| nonempty(&row.coach_name))?;
    let email = nonempty(&row.ad_email).or_else(|| nonempty(&row.public_professional_email));
    Some(ad_entity(
        school_id,
        strip_honorific(&ad_name),
        email,
        source,
    ))
}

/// The athletic office a coaching row's AD columns describe.
fn imported_ad(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
) -> Option<CanonicalCoach> {
    let ad_name = nonempty(&row.ad_name)?;
    Some(ad_entity(
        school_id,
        strip_honorific(&ad_name),
        nonempty(&row.ad_email),
        source,
    ))
}

/// Build the athletic-director coach entity both AD branches share.
fn ad_entity(
    school_id: &SchoolId,
    name: String,
    email: Option<String>,
    source: &RowSource,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(
        school_id,
        name,
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    );
    attach_source(&mut coach, format!("{}:ad", source.key), email, source);
    coach
}

/// Import a contact CSV, writing canonical schools and coaches into the store.
///
/// Duplicate coach rows (the same AD repeated across a school's sport rows) collapse to one entity
/// with the union of its evidence and the first non-empty email.
pub fn import_csv(
    store: &Store,
    csv_path: &Path,
    default_observed_on: &str,
) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("coach_contacts_csv", "coaches");
    let file = std::fs::File::open(csv_path)
        .with_context(|| format!("opening contact csv {}", csv_path.display()))?;
    let mut reader = csv::Reader::from_reader(file);

    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    let mut coaches: BTreeMap<CoachId, CanonicalCoach> = BTreeMap::new();
    let mut skipped_roles = 0usize;

    for (index, record) in reader.deserialize::<CoachContactRow>().enumerate() {
        let row = contact_row(record, index, csv_path)?;
        let entities = row_entities(&row, default_observed_on)?;
        if merge_entities(&mut schools, &mut coaches, entities) {
            skipped_roles = skipped_roles.saturating_add(1);
        }
    }
    let _ = Table::Coaches;

    write_entities(store, &mut report, schools, coaches, skipped_roles)?;
    Ok(report)
}

/// Deserialize one CSV record, rejecting a row with no school or state.
fn contact_row(
    record: Result<CoachContactRow, csv::Error>,
    index: usize,
    csv_path: &Path,
) -> Result<CoachContactRow> {
    let row = record
        .with_context(|| format!("row {} of {}", index.saturating_add(2), csv_path.display()))?;
    if row.school.trim().is_empty() || row.state.trim().is_empty() {
        anyhow::bail!("row {} has no school/state", index.saturating_add(2));
    }
    Ok(row)
}

/// Fold one row's entities into the accumulated schools and coaches.
///
/// Returns `true` when the row carried no coach role.
fn merge_entities(
    schools: &mut BTreeMap<String, CanonicalSchool>,
    coaches: &mut BTreeMap<CoachId, CanonicalCoach>,
    entities: RowEntities,
) -> bool {
    let school_id = entities.school.id.clone();
    schools
        .entry(school_id.as_str().to_string())
        .or_insert(entities.school);
    let without_coach_role = entities.coaches.is_empty();
    for coach in entities.coaches {
        merge_coach(coaches, coach);
    }
    without_coach_role
}

/// Union one coach row into the map entry it shares an identity with.
fn merge_coach(coaches: &mut BTreeMap<CoachId, CanonicalCoach>, coach: CanonicalCoach) {
    match coaches.get_mut(&coach.id) {
        Some(existing) => {
            if existing.professional_email.is_none() {
                existing.professional_email = coach.professional_email.clone();
            }
            for evidence in coach.evidence.iter().cloned() {
                if !existing.evidence.contains(&evidence) {
                    existing.evidence.push(evidence);
                }
            }
            for identity in coach.source_identities.iter().cloned() {
                if !existing.source_identities.contains(&identity) {
                    existing.source_identities.push(identity);
                }
            }
        }
        None => {
            coaches.insert(coach.id.clone(), coach);
        }
    }
}

/// Write the accumulated entities and fill in the report's counters and notes.
fn write_entities(
    store: &Store,
    report: &mut AdapterReport,
    schools: BTreeMap<String, CanonicalSchool>,
    coaches: BTreeMap<CoachId, CanonicalCoach>,
    skipped_roles: usize,
) -> Result<()> {
    let school_records: Vec<CanonicalSchool> = schools.into_values().collect();
    let coach_records: Vec<CanonicalCoach> = coaches.into_values().collect();
    store.append_many(Table::Schools, &school_records)?;
    store.append_many(Table::Coaches, &coach_records)?;

    report.rows = u64::try_from(coach_records.len()).unwrap_or(u64::MAX);
    report.with_email = coach_records
        .iter()
        .filter(|coach| coach.professional_email.is_some())
        .count()
        .try_into()
        .unwrap_or(u64::MAX);
    report.note(format!("schools={}", school_records.len()));
    report.note(format!("rows_without_coach_role={skipped_roles}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Table;

    const CSV: &str = include_str!("../../tests/fixtures/coach_contacts_sample.csv");

    fn rows() -> Vec<CoachContactRow> {
        let mut reader = csv::Reader::from_reader(CSV.as_bytes());
        reader
            .deserialize::<CoachContactRow>()
            .map(|row| row.unwrap())
            .collect()
    }

    #[test]
    fn parses_sport_and_gender_labels() {
        assert_eq!(
            parse_sport("Boys Track and Field"),
            Some((Sport::OutdoorTrack, Gender::Boys))
        );
        assert_eq!(
            parse_sport("Varsity Head Coach - Girls Cross Country"),
            Some((Sport::CrossCountry, Gender::Girls))
        );
        assert_eq!(
            parse_sport("Girls Track & Field Head Coach"),
            Some((Sport::OutdoorTrack, Gender::Girls))
        );
        assert_eq!(parse_sport(""), None);
    }

    #[test]
    fn non_coaching_roles_are_not_imported() {
        assert_eq!(parse_role("Athletic Director Secretary"), None);
        assert_eq!(parse_role("Principal"), None);
        assert_eq!(parse_role("Superintendent"), None);
        assert_eq!(parse_role("Athletic Trainer"), None);
        assert_eq!(
            parse_role("Varsity Head Coach - Boys Cross Country"),
            Some(CoachRole::HeadCoach)
        );
        assert_eq!(
            parse_role("Varsity Assistant Coach - Girls Track & Field"),
            Some(CoachRole::AssistantCoach)
        );
        assert_eq!(
            parse_role("Activities Director"),
            Some(CoachRole::AthleticDirector)
        );
    }

    #[test]
    fn coach_rows_become_canonical_entities_with_evidence() {
        let rows = rows();
        let wiaa = rows
            .iter()
            .find(|row| row.school == "Abbotsford")
            .expect("fixture row");
        let entities = row_entities(wiaa, "2026-09-20").unwrap();
        assert_eq!(entities.school.state.as_deref(), Some("WI"));
        assert_eq!(entities.school.city.as_deref(), Some("Abbotsford"));
        assert_eq!(entities.school.name, "Abbotsford");
        // one sport coach + one AD
        assert_eq!(entities.coaches.len(), 2);
        let coach = entities
            .coaches
            .iter()
            .find(|coach| coach.role == CoachRole::HeadCoach)
            .unwrap();
        assert_eq!(coach.name, "JACOB KNAPMILLER");
        assert_eq!(coach.sport, Some(Sport::OutdoorTrack));
        assert_eq!(coach.gender, Gender::Boys);
        assert_eq!(
            coach.professional_email.as_deref(),
            Some("jknapmiller@abbotsford.k12.wi.us")
        );
        let ad = entities
            .coaches
            .iter()
            .find(|coach| coach.role == CoachRole::AthleticDirector)
            .unwrap();
        // Athletic directors are school-wide: no sport binding.
        assert_eq!(ad.sport, None);
        assert_eq!(
            ad.professional_email.as_deref(),
            Some("alarson@abbotsford.k12.wi.us")
        );
        assert!(coach.evidence.iter().all(|evidence| evidence
            .source
            .url
            .as_deref()
            .unwrap()
            .contains("orgID=1")));
    }

    #[test]
    fn import_dedupes_schools_and_ad_rows() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let path = dir.path().join("coach-contacts.csv");
        std::fs::write(&path, CSV).unwrap();
        let report = import_csv(&store, &path, "2026-09-20").unwrap();
        assert!(report.rows > 0, "coach rows imported: {}", report.rows);

        let read_schools = || -> Vec<serde_json::Value> {
            store
                .scan::<CanonicalSchool>(Table::Schools)
                .unwrap()
                .into_iter()
                .map(|v| serde_json::to_value(&v).unwrap())
                .collect()
        };
        let read_coaches = || -> Vec<serde_json::Value> {
            store
                .scan::<CanonicalCoach>(Table::Coaches)
                .unwrap()
                .into_iter()
                .map(|v| serde_json::to_value(&v).unwrap())
                .collect()
        };
        let schools = read_schools();
        let coaches = read_coaches();

        // Four Abbotsford rows collapse to one school record; the MI slug/name forms stay distinct
        // from each other but each is a single record.
        let abbotsford = schools
            .iter()
            .filter(|school| school["name"] == "Abbotsford")
            .count();
        assert_eq!(
            abbotsford, 1,
            "duplicate school rows collapse to one entity"
        );
        let school_names: Vec<String> = schools
            .iter()
            .map(|school| school["name"].as_str().unwrap().to_string())
            .collect();
        for expected in [
            "Abilene HS",
            "aberdeencentral",
            "Adams Central",
            "Abingdon-Avon High School",
        ] {
            assert!(
                school_names.iter().any(|name| name == expected),
                "missing school {expected} in {school_names:?}"
            );
        }

        // Non-coaching office staff are dropped even though they occupy the `ad_name` column.
        let coach_names: Vec<String> = coaches
            .iter()
            .map(|coach| coach["name"].as_str().unwrap().to_string())
            .collect();
        for dropped in ["Kevin Polston", "Omar Bakri", "Mindy Langlois"] {
            assert!(
                !coach_names.iter().any(|name| name == dropped),
                "non-coaching office staff imported: {dropped}"
            );
        }
        // The SD AD appears on two duplicate rows and must exist exactly once.
        assert_eq!(
            coach_names.iter().filter(|name| *name == "Bo Beck").count(),
            1,
            "duplicate AD rows collapse: {coach_names:?}"
        );
        // Honorifics are stripped so the IL coach matches a plain-name observation.
        assert!(
            coach_names.iter().any(|name| name == "Barry Mink"),
            "{coach_names:?}"
        );
        assert!(report.with_email > 0);
    }
}
