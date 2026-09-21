//! Athletic.net athlete-bio adapter — the owner-authorized Athletic.net source.
//!
//! # Access
//!
//! Athletic.net's robots policy (fetched 2026-09-21, the whole 40-line file) allows `/api/` for
//! `User-agent: *` and disallows `/Search.aspx`, and the host answers the collector's **own** user
//! agent with `200` — so this adapter identifies itself, sends no spoofed browser headers, and
//! never touches the disallowed search endpoint. Athlete ids therefore cannot be discovered here:
//! they arrive from the operator's registry file (`--input`), one `athlete_id` or
//! `athlete_id,ST` per line.
//!
//! # Endpoint contract (verified against live responses, 2026-09-21)
//!
//! `GET https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=<id>&sport=<tf|xc>&level=4`
//!
//! | payload | `sport=tf` | `sport=xc` |
//! |---|---|---|
//! | `resultsTF` | every track & field result | empty |
//! | `resultsXC` | absent | every cross-country result |
//! | `eventsTF` | `IDEvent` → event label | null |
//! | `meets` | `IDMeet` → `{MeetName, EndDate}` for its own rows | same, for its own rows |
//! | `grades` | `"<SchoolID>_<SeasonID>"` → grade | same |
//! | `allTeams` | `SchoolID` → `{IDSchool, SchoolName, …}` | same |
//! | `allSeasons` | `[{SchoolID, IDSeason, Display}]` | same |
//!
//! The two calls are not redundant and neither is optional: an athlete with 40 track results and 22
//! cross-country results returns only the first under `sport=tf` and only the second under
//! `sport=xc`. Nothing in either payload carries the indoor/outdoor split except
//! `allSeasons[].Display` (`"2026 Indoor"`, `"2026 Outdoor"`).
//!
//! # What this adapter refuses to guess
//!
//! A row is skipped — and counted, and named in the run report — when its season has no
//! `allSeasons` entry (the indoor/outdoor split is then unpublished), when its school has no
//! `allTeams` entry, when its meet has no `meets` entry, when the target carries no state (school
//! identity keys on state + name, and `"Springfield"` in two states is two schools), or when the
//! mark is a no-mark token. The refusal is the point: a mislabelled indoor/outdoor row is worse
//! than a counted gap.

use crate::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, Grade, Mark,
    ObservedGrade, SchoolId, SchoolYear, SourceEventLabel, SourceIdentity, SourceNamespace,
    SourceRef, Sport, TimingMethod,
};
use crate::net::FetchOptions;
use crate::school_index::SchoolIndex;
use crate::sources::hytek::{parse_field_mark, parse_time, NO_MARK};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Athlete bio endpoint; `sport` (`tf`/`xc`), `athleteId` and `level` are its parameters.
const BIO_ENDPOINT: &str = "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData";

/// Athletic.net's "high school" level selector.
const HIGH_SCHOOL_LEVEL: u32 = 4;

/// Bump when a parser change alters what an already-journaled bio yields.
const PARSE_VERSION: u32 = 1;

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Registry file: one `athlete_id` or `athlete_id,ST` per line; `#` comments and blank lines are
    /// ignored. The operator supplies it because the search endpoint is disallowed by robots.
    pub input: Option<String>,
    /// Cap the number of athletes processed (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// State applied to targets that carry none. Only unambiguous for a single-state batch.
    pub states: Vec<String>,
}

/// One athlete to read, with the state that disambiguates its school.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub athlete_id: u64,
    pub state: Option<String>,
}

/// Parse the operator's athlete registry.
///
/// Lines are `athlete_id` or `athlete_id,ST`; a single `--states` value fills in the state for
/// targets that name none, and two or more are refused as ambiguous rather than guessed between.
pub fn parse_targets(body: &str, default_states: &[String]) -> Result<Vec<Target>> {
    ensure!(
        default_states.len() <= 1,
        "a registry without a per-line state needs exactly one --states value, got {} ({})",
        default_states.len(),
        default_states.join(",")
    );
    let fallback = default_states
        .first()
        .map(|state| state.trim().to_ascii_uppercase())
        .filter(|state| !state.is_empty());
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    for (number, line) in body.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split([',', '\t', ' ']).filter(|part| !part.is_empty());
        let id_text = parts.next().unwrap_or_default();
        let athlete_id: u64 = id_text.parse().with_context(|| {
            format!(
                "registry line {}: `{id_text}` is not an athlete id",
                number + 1
            )
        })?;
        let state = parts
            .next()
            .map(|state| state.trim().to_ascii_uppercase())
            .or_else(|| fallback.clone());
        ensure!(
            parts.next().is_none(),
            "registry line {}: expected `athlete_id[,ST]`, got `{line}`",
            number + 1
        );
        if let Some(state) = &state {
            ensure!(
                state.len() == 2 && state.chars().all(|c| c.is_ascii_alphabetic()),
                "registry line {}: `{state}` is not a two-letter state code",
                number + 1
            );
        }
        // A registry may legitimately repeat an athlete across state files; read it once.
        if seen.insert(athlete_id) {
            targets.push(Target { athlete_id, state });
        }
    }
    Ok(targets)
}

// -------------------------------------------------------------------------------------------------
// Wire format
// -------------------------------------------------------------------------------------------------

/// One athlete bio payload. Fields the adapter does not read are not declared.
#[derive(Debug, Clone, Deserialize)]
pub struct Bio {
    pub athlete: BioAthlete,
    #[serde(default)]
    pub grades: Option<BTreeMap<String, i64>>,
    #[serde(default, rename = "allTeams")]
    pub teams: BTreeMap<String, BioTeam>,
    #[serde(default, rename = "allSeasons")]
    pub seasons: Vec<BioSeason>,
    #[serde(default, rename = "eventsTF")]
    pub events: Option<Vec<BioEvent>>,
    #[serde(default, rename = "resultsTF")]
    pub results_tf: Option<Vec<TfRow>>,
    #[serde(default, rename = "resultsXC")]
    pub results_xc: Option<Vec<XcRow>>,
    #[serde(default)]
    pub meets: BTreeMap<String, BioMeet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioAthlete {
    #[serde(rename = "IDAthlete")]
    pub id: u64,
    #[serde(rename = "FirstName", default)]
    pub first_name: String,
    #[serde(rename = "LastName", default)]
    pub last_name: String,
    /// `"M"` or `"F"`.
    #[serde(rename = "Gender", default)]
    pub gender: String,
    #[serde(rename = "SchoolID", default)]
    pub school_id: Option<i64>,
}

impl BioAthlete {
    /// `"First Last"`, trimmed; empty when the payload names nobody.
    pub fn name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim())
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioTeam {
    #[serde(rename = "SchoolName", default)]
    pub school_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioSeason {
    #[serde(rename = "SchoolID", default)]
    pub school_id: i64,
    #[serde(rename = "IDSeason")]
    pub season_id: i16,
    /// `"2026 Indoor"`, `"2026 Outdoor"`, `"2026 Cross Country"`.
    #[serde(rename = "Display", default)]
    pub display: String,
}

impl BioSeason {
    /// The indoor/outdoor split, or `None` when the display does not publish one.
    fn sport(&self) -> Option<Sport> {
        let display = self.display.to_ascii_lowercase();
        if display.contains("indoor") {
            Some(Sport::IndoorTrack)
        } else if display.contains("outdoor") {
            Some(Sport::OutdoorTrack)
        } else if display.contains("cross") {
            Some(Sport::CrossCountry)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioEvent {
    #[serde(rename = "IDEvent")]
    pub id: i64,
    #[serde(rename = "Event", default)]
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BioMeet {
    #[serde(rename = "MeetName", default)]
    pub name: String,
    /// `"2025-05-03T00:00:00"`.
    #[serde(rename = "EndDate", default)]
    pub end_date: String,
}

impl BioMeet {
    fn date(&self) -> Option<&str> {
        let date = self.end_date.split('T').next().unwrap_or_default().trim();
        (date.len() == 10 && date.as_bytes().get(4) == Some(&b'-')).then_some(date)
    }
}

/// One track & field result.
#[derive(Debug, Clone, Deserialize)]
pub struct TfRow {
    #[serde(rename = "IDResult")]
    pub id: i64,
    /// `"1:17.80a"` (auto-timed, trailing `a`), `"11.32a"`, `"20.51m"`, `"DNS"`.
    #[serde(rename = "Result", default)]
    pub result: String,
    /// `1` for fully automatic timing.
    #[serde(rename = "FAT", default)]
    pub fat: i64,
    /// Published as a string here and as a number in cross country.
    #[serde(rename = "Place", default, deserialize_with = "optional_text")]
    pub place: Option<String>,
    #[serde(rename = "Round", default)]
    pub round: Option<String>,
    #[serde(rename = "Wind", default)]
    pub wind: Option<f64>,
    #[serde(rename = "Division", default)]
    pub division: Option<String>,
    #[serde(rename = "SchoolID", default)]
    pub school_id: Option<i64>,
    #[serde(rename = "EventID", default)]
    pub event_id: Option<i64>,
    #[serde(rename = "MeetID", default)]
    pub meet_id: Option<i64>,
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i16>,
    /// `"2025-05-02T00:00:00"`.
    #[serde(rename = "ResultDate", default)]
    pub result_date: Option<String>,
}

impl TfRow {
    fn date(&self) -> Option<String> {
        self.result_date
            .as_deref()
            .and_then(|raw| raw.split('T').next())
            .map(str::trim)
            .filter(|date| date.len() == 10)
            .map(str::to_string)
    }
}

/// One cross-country result. Cross country publishes no event id (the distance replaces it) and no
/// result date (the meet's end date is the published date).
#[derive(Debug, Clone, Deserialize)]
pub struct XcRow {
    #[serde(rename = "IDResult")]
    pub id: i64,
    #[serde(rename = "Result", default)]
    pub result: String,
    #[serde(rename = "Place", default, deserialize_with = "optional_text")]
    pub place: Option<String>,
    #[serde(rename = "Division", default)]
    pub division: Option<String>,
    #[serde(rename = "SchoolID", default)]
    pub school_id: Option<i64>,
    #[serde(rename = "MeetID", default)]
    pub meet_id: Option<i64>,
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i16>,
    /// Course distance in metres (`5000`), published beside the cross-country time.
    #[serde(rename = "Distance", default)]
    pub distance: Option<i64>,
}

/// `Place` is a string on the track payload and a number on the cross-country one.
fn optional_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match value {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(text)) => {
            let text = text.trim().to_string();
            (!text.is_empty()).then_some(text)
        }
        Some(serde_json::Value::Number(number)) => Some(number.to_string()),
        Some(other) => return Err(D::Error::custom(format!("unexpected place value {other}"))),
    })
}

// -------------------------------------------------------------------------------------------------
// Marks
// -------------------------------------------------------------------------------------------------

/// The mark a published result token denotes, plus whether it was fully automatic, or `None` when
/// the token is not a mark at all.
///
/// Athletic.net publishes times and field marks with a trailing `a` when the mark is automatic,
/// qualifier suffixes (`q`/`Q`/`p`/`P`) beside the mark, and no-mark words (`DNS`, `ND`, `FOUL`) in
/// the same column. A no-mark row yields no performance.
pub fn parse_mark(kind: &EventKind, published: &str) -> Option<(Mark, bool)> {
    let trimmed = published.trim();
    if trimmed.is_empty() || NO_MARK.contains(&trimmed.to_ascii_uppercase().as_str()) {
        return None;
    }
    let auto = trimmed.ends_with('a') || trimmed.ends_with('A');
    let token = trimmed
        .trim_end_matches(['a', 'A', 'q', 'Q', 'p', 'P'])
        .trim();
    if !token.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let mark = match kind {
        EventKind::Pentathlon | EventKind::Heptathlon | EventKind::Decathlon => {
            Mark::Points(token.replace(',', "").parse().ok()?)
        }
        kind if kind.is_field() => parse_field_mark(metric_bare(token))?,
        _ => Mark::TimeSeconds(parse_time(token)?),
    };
    Some((mark, auto))
}

/// Drop the `m` Athletic.net prints on metric field marks (`12.34m`), so the shared field parser
/// sees the bare figure it expects. Only a token that is otherwise a plain number is shortened.
fn metric_bare(token: &str) -> &str {
    match token.strip_suffix(['m', 'M']) {
        Some(head) if head.trim().parse::<f64>().is_ok() => head.trim(),
        _ => token,
    }
}

/// Timing method: the published automatic-timing flag and the `a` suffix both mean fully
/// automatic; a bare mark is hand-timed.
fn timing_of(fat: i64, auto: bool) -> Option<TimingMethod> {
    if fat == 1 || auto {
        Some(TimingMethod::Fat)
    } else {
        Some(TimingMethod::Hand)
    }
}

fn round_of(published: Option<&str>) -> Option<String> {
    let published = published?.trim();
    match published.to_ascii_uppercase().as_str() {
        "" => None,
        "F" => Some("final".to_string()),
        "P" => Some("prelim".to_string()),
        "S" => Some("semifinal".to_string()),
        other => Some(other.to_ascii_lowercase()),
    }
}

fn gender_of(published: &str) -> Option<Gender> {
    match published.trim().to_ascii_uppercase().as_str() {
        "F" => Some(Gender::Girls),
        "M" => Some(Gender::Boys),
        _ => None,
    }
}

/// Profile URL for an athlete id, in the form Athletic.net itself links to.
fn profile_url(athlete_id: u64) -> String {
    format!("https://www.athletic.net/athlete/{athlete_id}/track-and-field")
}

// -------------------------------------------------------------------------------------------------
// Collection
// -------------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Stats {
    athletes_seen: u64,
    athletes_absorbed: u64,
    athletes_without_grade: u64,
    athletes_without_school: u64,
    gender_unknown: u64,
    rows_seen: u64,
    rows_absorbed: u64,
    rows_no_mark: u64,
    rows_no_season: u64,
    rows_unknown_season: HashMap<String, u64>,
    rows_no_event: u64,
    rows_unknown_school: u64,
    rows_unknown_meet: u64,
    rows_without_state: u64,
    schools_minted: u64,
    schools_resolved: u64,
    fetches_failed: u64,
}

#[derive(Default)]
struct Accumulator {
    schools: HashMap<String, CanonicalSchool>,
    meets: HashMap<String, CanonicalMeet>,
    teams: HashMap<String, CanonicalTeam>,
    athletes: HashMap<String, CanonicalAthlete>,
    events: HashMap<String, CanonicalEvent>,
    performances: HashMap<String, CanonicalPerformance>,
}

/// Which endpoint answered: track & field, or cross country.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    TrackField,
    CrossCountry,
}

impl Scope {
    fn parameter(self) -> &'static str {
        match self {
            Scope::TrackField => "tf",
            Scope::CrossCountry => "xc",
        }
    }
}

const SCOPES: [Scope; 2] = [Scope::TrackField, Scope::CrossCountry];

/// Read every available result for the registry's athletes into the canonical store.
///
/// Strategy: one request per (athlete, sport) pair, journaled per URL so a re-run resumes; both
/// payloads are absorbed under one athlete so its teams and grades are minted once.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("athleticnet", "athletes");
    let (requests_before, cache_before) = stats_of(ctx).await;

    let input = options.input.as_deref().context(
        "the athletic.net adapter needs --input with an athlete registry; its search endpoint is \
         disallowed by robots, so ids cannot be discovered by the tool",
    )?;
    let body = std::fs::read_to_string(input)
        .with_context(|| format!("reading the athlete registry {input}"))?;
    let targets = parse_targets(&body, &options.states)?;
    ensure!(
        !targets.is_empty(),
        "the athlete registry {input} lists no athlete ids"
    );
    let without_state = targets.iter().filter(|t| t.state.is_none()).count();
    if without_state > 0 {
        report.note(format!(
            "{without_state} of {} targets carry no state; their rows are skipped, because a school \
             key without a state would merge same-named schools across states",
            targets.len()
        ));
    }

    let index = consolidated_index(ctx)?;
    // One resolution per (state, school) for the whole run, so the counts are per school.
    let mut resolved: HashMap<String, SchoolId> = HashMap::new();
    let source = SourceRef::new("athleticnet", Some(BIO_ENDPOINT.to_string()));
    let done: HashSet<String> = ctx
        .store
        .journal_payloads("athleticnet")?
        .into_iter()
        .filter(|entry| {
            entry.get("parser").and_then(serde_json::Value::as_u64)
                == Some(u64::from(PARSE_VERSION))
                && entry.get("parsed").and_then(serde_json::Value::as_bool) == Some(true)
        })
        .filter_map(|entry| {
            entry
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();

    let mut stats = Stats::default();
    let mut accumulated = Accumulator::default();
    for (processed, target) in targets.iter().enumerate() {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        stats.athletes_seen += 1;
        let mut absorbed_any = false;
        for scope in SCOPES {
            let url = format!(
                "{BIO_ENDPOINT}?athleteId={}&sport={}&level={HIGH_SCHOOL_LEVEL}",
                target.athlete_id,
                scope.parameter()
            );
            if done.contains(&url) {
                continue;
            }
            let fetch_options = FetchOptions {
                refresh: options.refresh,
                allow_not_found: false,
                headers: vec![("Accept".to_string(), "application/json".to_string())],
            };
            let fetched = match ctx.fetcher.get(&url, &fetch_options).await {
                Ok(fetched) => fetched,
                Err(error) => {
                    stats.fetches_failed += 1;
                    report.note(format!("athlete {}: {error}", target.athlete_id));
                    continue;
                }
            };
            let bio: Bio = match serde_json::from_str(&fetched.text()) {
                Ok(bio) => bio,
                Err(error) => {
                    stats.fetches_failed += 1;
                    report.note(format!(
                        "athlete {} {}: body is not an athlete bio ({error})",
                        target.athlete_id,
                        scope.parameter()
                    ));
                    continue;
                }
            };
            let rows = absorb(
                &bio,
                scope,
                target,
                &source,
                &options.observed_on,
                &index,
                &mut resolved,
                &mut stats,
                &mut accumulated,
            );
            absorbed_any |= rows > 0;
            ctx.store.journal_done(
                "athleticnet",
                &url,
                &json!({
                    "url": url,
                    "parser": PARSE_VERSION,
                    "parsed": true,
                    "athlete": target.athlete_id,
                    "sport": scope.parameter(),
                    "rows": rows,
                }),
            )?;
        }
        if absorbed_any {
            stats.athletes_absorbed += 1;
        }
    }

    let schools: Vec<CanonicalSchool> = accumulated.schools.into_values().collect();
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    ctx.store.append_many(Table::Schools, &schools)?;
    ctx.store.append_many(Table::Meets, &meets)?;
    ctx.store.append_many(Table::Teams, &teams)?;
    ctx.store.append_many(Table::Athletes, &athletes)?;
    ctx.store.append_many(Table::Events, &events)?;
    ctx.store.append_many(Table::Performances, &performances)?;

    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = stats.athletes_absorbed;
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.errors = stats.fetches_failed;
    report.note(format!(
        "athletes: {} seen, {} absorbed, {} without a published grade, {} without a school entry, \
         {} whose gender the payload does not publish",
        stats.athletes_seen,
        stats.athletes_absorbed,
        stats.athletes_without_grade,
        stats.athletes_without_school,
        stats.gender_unknown
    ));
    report.note(format!(
        "result rows: {} seen, {} absorbed, {} without a mark token",
        stats.rows_seen, stats.rows_absorbed, stats.rows_no_mark
    ));
    report.note(format!(
        "skipped rows: {} with no season id, {} whose season publishes no indoor/outdoor split {}, \
         {} whose event id is absent from the payload's event dictionary, {} without a school \
         entry, {} without a meet entry, {} whose target carries no state",
        stats.rows_no_season,
        stats.rows_unknown_season.values().sum::<u64>(),
        if stats.rows_unknown_season.is_empty() {
            String::new()
        } else {
            format!("{:?}", stats.rows_unknown_season)
        },
        stats.rows_no_event,
        stats.rows_unknown_school,
        stats.rows_unknown_meet,
        stats.rows_without_state
    ));
    report.note(format!(
        "schools: {} resolved against the consolidated index, {} minted from this source",
        stats.schools_resolved, stats.schools_minted
    ));
    report.note(format!(
        "canonical entities: schools {} meets {} teams {} athletes {} events {} performances {}",
        schools.len(),
        meets.len(),
        teams.len(),
        athletes.len(),
        events.len(),
        performances.len()
    ));
    report.note(
        "this source is outside the core scope (`report --core`): it is a reseller of results the \
         platform also gathers from governing bodies and timers, so the core comparison stays \
         independent of it"
            .to_string(),
    );
    Ok(report)
}

/// The consolidated school index, when one exists. Athletic.net spans the whole country while the
/// index covers the platform's states, so a miss mints rather than skips.
fn consolidated_index(ctx: &AdapterContext<'_>) -> Result<SchoolIndex> {
    let path = ctx.store.out_dir().join("schools.jsonl");
    if !path.exists() {
        return Ok(SchoolIndex::from_schools(&[]));
    }
    let schools: Vec<CanonicalSchool> = crate::report::read_rows(&path)?;
    Ok(SchoolIndex::from_schools(&schools))
}

/// Absorb one payload; returns the number of result rows stored.
#[allow(clippy::too_many_arguments)]
fn absorb(
    bio: &Bio,
    scope: Scope,
    target: &Target,
    source: &SourceRef,
    observed_on: &str,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, SchoolId>,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> u64 {
    let name = bio.athlete.name();
    let Some(gender) = gender_of(&bio.athlete.gender) else {
        stats.gender_unknown += 1;
        return 0;
    };

    // Grade observations: `grades` maps "<SchoolID>_<SeasonID>" to the grade that season, which is
    // what makes a class year derived rather than assumed.
    let mut observed_grades: Vec<ObservedGrade> = Vec::new();
    for (key, grade) in bio.grades.iter().flatten() {
        let Some((_, season)) = key.split_once('_') else {
            continue;
        };
        let (Ok(grade), Ok(season)) = (u8::try_from(*grade), season.parse::<i16>()) else {
            continue;
        };
        let Some(grade) = Grade::new(grade) else {
            continue;
        };
        observed_grades.push(ObservedGrade {
            grade,
            school_year: SchoolYear::containing(season, 5),
            source: source.clone(),
        });
    }
    observed_grades.sort_by_key(|observation| observation.school_year);
    let Some(latest) = observed_grades.last().cloned() else {
        stats.athletes_without_grade += 1;
        return 0;
    };
    let grad_year = latest.grad_year();

    let Some(athlete_school) = bio.athlete.school_id else {
        stats.athletes_without_school += 1;
        return 0;
    };
    // School names are published once per payload, keyed by the school id the rows carry.
    let school_names: HashMap<String, &str> = bio
        .teams
        .iter()
        .map(|(id, team)| (id.clone(), team.school_name.as_str()))
        .collect();
    let Some(school) = school_for(
        &athlete_school.to_string(),
        target.state.as_deref(),
        &school_names,
        index,
        resolved,
        source,
        observed_on,
        stats,
        accumulated,
    ) else {
        stats.rows_without_state += 1;
        return 0;
    };

    let athlete_id: AthleteId =
        match accumulated
            .athletes
            .get(&format!("{}:{}", target.athlete_id, school.as_str()))
        {
            Some(existing) => existing.id.clone(),
            None => {
                let mut athlete = CanonicalAthlete::new(&school, &name, grad_year, gender);
                athlete
                    .public_profile_urls
                    .push(profile_url(target.athlete_id));
                let id = athlete.id.clone();
                accumulated.athletes.insert(
                    format!("{}:{}", target.athlete_id, school.as_str()),
                    athlete,
                );
                id
            }
        };
    if let Some(athlete) =
        accumulated
            .athletes
            .get_mut(&format!("{}:{}", target.athlete_id, school.as_str()))
    {
        athlete.observed_grades = observed_grades.clone();
        athlete.evidence = vec![Evidence::fetched(source.clone(), observed_on)];
        athlete.source_identities = vec![SourceIdentity {
            namespace: SourceNamespace::AthleticNet {
                kind: "athlete".to_string(),
            },
            id: target.athlete_id.to_string(),
            url: Some(profile_url(target.athlete_id)),
        }];
    }

    // Season entries are the only place the indoor/outdoor split is published, keyed by the
    // athlete's school + season.
    let seasons: HashMap<(i64, i16), Option<Sport>> = bio
        .seasons
        .iter()
        .map(|season| ((season.school_id, season.season_id), season.sport()))
        .collect();

    let mut rows = 0u64;
    match scope {
        Scope::TrackField => {
            let labels: HashMap<i64, &str> = bio
                .events
                .iter()
                .flatten()
                .map(|event| (event.id, event.label.as_str()))
                .collect();
            for row in bio.results_tf.iter().flatten() {
                stats.rows_seen += 1;
                let Some(season_id) = row.season_id else {
                    stats.rows_no_season += 1;
                    continue;
                };
                let Some(sport) = seasons
                    .get(&(row.school_id.unwrap_or_default(), season_id))
                    .copied()
                    .flatten()
                else {
                    *stats
                        .rows_unknown_season
                        .entry(season_id.to_string())
                        .or_default() += 1;
                    continue;
                };
                let Some(label) = row.event_id.and_then(|id| labels.get(&id).copied()) else {
                    stats.rows_no_event += 1;
                    continue;
                };
                let kind = EventKind::from_source_label(label);
                let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
                    stats.rows_no_mark += 1;
                    continue;
                };
                let Some(meet) = meet_for(
                    row.meet_id,
                    bio,
                    target.state.as_deref(),
                    source,
                    observed_on,
                    accumulated,
                ) else {
                    stats.rows_unknown_meet += 1;
                    continue;
                };
                let Some(date) = row.date().or_else(|| Some(meet.date.clone())) else {
                    stats.rows_unknown_meet += 1;
                    continue;
                };
                let Some(school) = school_for(
                    &row.school_id.unwrap_or_default().to_string(),
                    target.state.as_deref(),
                    &school_names,
                    index,
                    resolved,
                    source,
                    observed_on,
                    stats,
                    accumulated,
                ) else {
                    stats.rows_without_state += 1;
                    continue;
                };
                let school_year = SchoolYear::containing(season_id, 5);
                let grade = grade_in(&observed_grades, school_year);
                store_performance(
                    accumulated,
                    source,
                    observed_on,
                    PerformanceInput {
                        athlete: &athlete_id,
                        school: &school,
                        meet: &meet,
                        kind: &kind,
                        sport,
                        gender,
                        school_year,
                        grade,
                        date,
                        mark,
                        wind_mps: row.wind,
                        place: row.place.as_deref(),
                        round: round_of(row.round.as_deref()),
                        timing: timing_of(row.fat, auto),
                        division: row.division.clone(),
                        source_key: format!("athleticnet:{}-{}", target.athlete_id, row.id),
                        label: Some(label),
                    },
                );
                rows += 1;
                stats.rows_absorbed += 1;
            }
        }
        Scope::CrossCountry => {
            for row in bio.results_xc.iter().flatten() {
                stats.rows_seen += 1;
                let Some(season_id) = row.season_id else {
                    stats.rows_no_season += 1;
                    continue;
                };
                let kind = EventKind::CrossCountry;
                let Some((mark, auto)) = parse_mark(&kind, &row.result) else {
                    stats.rows_no_mark += 1;
                    continue;
                };
                let Some(meet) = meet_for(
                    row.meet_id,
                    bio,
                    target.state.as_deref(),
                    source,
                    observed_on,
                    accumulated,
                ) else {
                    stats.rows_unknown_meet += 1;
                    continue;
                };
                let date = meet.date.clone();
                let Some(school) = school_for(
                    &row.school_id.unwrap_or_default().to_string(),
                    target.state.as_deref(),
                    &school_names,
                    index,
                    resolved,
                    source,
                    observed_on,
                    stats,
                    accumulated,
                ) else {
                    stats.rows_without_state += 1;
                    continue;
                };
                // A cross-country season is a fall season, so its school year starts in the same
                // calendar year the season is named for.
                let school_year = SchoolYear::containing(season_id, 9);
                let grade = grade_in(&observed_grades, school_year);
                store_performance(
                    accumulated,
                    source,
                    observed_on,
                    PerformanceInput {
                        athlete: &athlete_id,
                        school: &school,
                        meet: &meet,
                        kind: &kind,
                        sport: Sport::CrossCountry,
                        gender,
                        school_year,
                        grade,
                        date,
                        mark,
                        wind_mps: None,
                        place: row.place.as_deref(),
                        round: None,
                        timing: timing_of(0, auto),
                        division: row
                            .division
                            .clone()
                            .or_else(|| row.distance.map(|metres| format!("{metres}m"))),
                        source_key: format!("athleticnet:{}-{}", target.athlete_id, row.id),
                        label: None,
                    },
                );
                rows += 1;
                stats.rows_absorbed += 1;
            }
        }
    }
    rows
}

/// The grade observed in a given school year, when the payload publishes one.
fn grade_in(observed: &[ObservedGrade], school_year: SchoolYear) -> Option<Grade> {
    observed
        .iter()
        .find(|observation| observation.school_year == school_year)
        .map(|observation| observation.grade)
}

/// Resolve or mint the school a payload row names, memoized per run so a school is counted once
/// rather than once per row that names it.
///
/// Returns `None` when the state is unknown or the payload publishes no name for the school id:
/// school identity keys on state + name, so minting without one would merge same-named schools in
/// different states.
#[allow(clippy::too_many_arguments)]
fn school_for(
    school_id: &str,
    state: Option<&str>,
    school_names: &HashMap<String, &str>,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, SchoolId>,
    source: &SourceRef,
    observed_on: &str,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> Option<SchoolId> {
    let state = state?;
    let key = format!("{state}:{school_id}");
    if let Some(id) = resolved.get(&key) {
        return Some(id.clone());
    }
    let Some(name) = school_names.get(school_id).map(|name| name.to_string()) else {
        stats.rows_unknown_school += 1;
        return None;
    };
    if let Some((id, _)) = index.resolve(state, &name) {
        stats.schools_resolved += 1;
        resolved.insert(key, id.clone());
        return Some(id);
    }
    let (mut school, id) = CanonicalSchool::new(state, name.clone(), name.to_lowercase());
    school.source_identities.push(SourceIdentity {
        namespace: SourceNamespace::AthleticNet {
            kind: "school".to_string(),
        },
        id: school_id.to_string(),
        url: None,
    });
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on));
    stats.schools_minted += 1;
    let id = accumulated
        .schools
        .entry(id.as_str().to_string())
        .or_insert(school)
        .id
        .clone();
    resolved.insert(key, id.clone());
    Some(id)
}

/// Resolve or mint the meet a payload row names.
fn meet_for(
    meet_id: Option<i64>,
    bio: &Bio,
    state: Option<&str>,
    source: &SourceRef,
    observed_on: &str,
    accumulated: &mut Accumulator,
) -> Option<CanonicalMeet> {
    let state = state?;
    let meet_id = meet_id?;
    let published = bio.meets.get(&meet_id.to_string())?;
    let date = published.date()?.to_string();
    let meet = accumulated
        .meets
        .entry(format!("{state}:{meet_id}"))
        .or_insert_with(|| {
            let mut meet = CanonicalMeet::new(
                state,
                published.name.clone(),
                date.clone(),
                CompetitionLevel::Unknown,
            );
            meet.source_identities.push(SourceIdentity {
                namespace: SourceNamespace::AthleticNet {
                    kind: "meet".to_string(),
                },
                id: meet_id.to_string(),
                url: None,
            });
            meet.evidence
                .push(Evidence::parsed(source.clone(), observed_on));
            meet
        });
    Some(meet.clone())
}

/// Everything one published result row contributes to a canonical performance.
struct PerformanceInput<'a> {
    athlete: &'a AthleteId,
    school: &'a SchoolId,
    meet: &'a CanonicalMeet,
    kind: &'a EventKind,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    grade: Option<Grade>,
    date: String,
    mark: Mark,
    wind_mps: Option<f64>,
    place: Option<&'a str>,
    round: Option<String>,
    timing: Option<TimingMethod>,
    division: Option<String>,
    source_key: String,
    label: Option<&'a str>,
}

/// Store one canonical performance, minting its team and event on the platform's keys.
#[allow(clippy::too_many_arguments)]
fn store_performance(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: PerformanceInput<'_>,
) {
    let team_id = CanonicalTeam::mint(input.school, input.sport, input.gender, input.school_year);
    if !accumulated.teams.contains_key(team_id.as_str()) {
        accumulated.teams.insert(
            team_id.as_str().to_string(),
            CanonicalTeam {
                id: team_id.clone(),
                school: input.school.clone(),
                sport: input.sport,
                gender: input.gender,
                school_year: input.school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![Evidence::parsed(source.clone(), observed_on)],
            },
        );
    }

    let event = accumulated
        .events
        .entry(format!(
            "{}:{:?}:{:?}:{}",
            input.meet.id.as_str(),
            input.kind,
            input.gender,
            input.division.clone().unwrap_or_default()
        ))
        .or_insert_with(|| {
            let mut event = CanonicalEvent::new(
                &input.meet.id,
                input.kind.clone(),
                input.gender,
                input.division.as_deref(),
                input.round.as_deref(),
            );
            if let Some(label) = input.label {
                event.source_labels.push(SourceEventLabel {
                    source: source.clone(),
                    label: label.to_string(),
                });
            }
            event
                .evidence
                .push(Evidence::parsed(source.clone(), observed_on));
            event
        })
        .id
        .clone();

    let performance_id = CanonicalPerformance::mint(
        input.athlete,
        &input.meet.id,
        input.kind,
        &input.date,
        &input.source_key,
    );
    accumulated
        .performances
        .entry(performance_id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id: performance_id,
            athlete: input.athlete.clone(),
            team: team_id,
            event,
            meet: input.meet.id.clone(),
            date: input.date,
            mark: input.mark,
            wind_mps: input.wind_mps,
            place: input.place.and_then(|place| place.trim().parse().ok()),
            heat: None,
            round: input.round,
            timing: input.timing,
            observed_grade: input.grade,
            evidence: vec![Evidence::parsed(source.clone(), observed_on)],
            source_key: input.source_key,
        });
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::is_core_source;

    fn payload(body: &str) -> Bio {
        serde_json::from_str(body).expect("a payload this adapter reads")
    }

    #[test]
    fn a_registry_line_carries_an_id_and_optionally_a_state() {
        let targets = parse_targets(
            "# season 2026\n28127170,AK\n\n26631105\n28127170,AK\n",
            &["wi".to_string()],
        )
        .expect("registry parses");
        assert_eq!(
            targets,
            vec![
                Target {
                    athlete_id: 28127170,
                    state: Some("AK".to_string())
                },
                Target {
                    athlete_id: 26631105,
                    state: Some("WI".to_string())
                },
            ],
            "the per-line state wins, the default fills a bare id, and a repeat reads once"
        );
    }

    #[test]
    fn a_registry_is_refused_rather_than_guessed_between_states() {
        let error = parse_targets("28127170\n", &["WI".to_string(), "AK".to_string()])
            .expect_err("two candidate states are ambiguous");
        assert!(
            error.to_string().contains("exactly one --states"),
            "the refusal names the fix: {error}"
        );
        assert!(parse_targets("28127170,Alaska\n", &[]).is_err());
        assert!(parse_targets("natalia\n", &[]).is_err());
        assert!(parse_targets("28127170,AK,extra\n", &[]).is_err());
    }

    #[test]
    fn marks_are_read_in_the_form_athleticnet_publishes() {
        let time = parse_mark(&EventKind::Track800m, "1:17.80a").expect("an auto-timed time");
        assert_eq!(time, (Mark::TimeSeconds(77.8), true));
        assert_eq!(
            parse_mark(&EventKind::Track3200m, "9:41.23"),
            Some((Mark::TimeSeconds(581.23), false)),
            "a bare mark is hand-timed"
        );
        assert_eq!(
            parse_mark(&EventKind::Track100m, "11.32q"),
            Some((Mark::TimeSeconds(11.32), false)),
            "a qualifier suffix is not part of the mark"
        );
        assert_eq!(
            parse_mark(&EventKind::LongJump, "5-04.25"),
            Some((
                Mark::FieldImperial {
                    feet_mark: "5-04.25".to_string(),
                    metres: 1.63195
                },
                false
            ))
        );
        assert_eq!(
            parse_mark(&EventKind::ShotPut, "12.34m"),
            Some((Mark::DistanceMetres(12.34), false)),
            "a metric field mark is metres, not a time"
        );
        assert_eq!(
            parse_mark(&EventKind::Decathlon, "3,456"),
            Some((Mark::Points(3456.0), false))
        );
    }

    #[test]
    fn a_no_mark_row_yields_no_performance() {
        for published in ["DNS", "ND", "FOUL", "X", ""] {
            assert_eq!(
                parse_mark(&EventKind::Track1600m, published),
                None,
                "`{published}` is not a mark"
            );
        }
    }

    #[test]
    fn cross_country_and_track_rows_deserialize_from_their_published_shapes() {
        // One TF row (string place, event id, result date), one XC row (numeric place, distance,
        // no result date), plus a null `resultsXC` — the three variations observed live.
        let bio = payload(
            r#"{
              "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
                          "Gender": "F", "SchoolID": 13850},
              "grades": {"13850_2026": 11},
              "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
              "allSeasons": [
                 {"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Indoor"},
                 {"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Outdoor"}
              ],
              "eventsTF": [{"IDEvent": 20, "Event": "200 Meters"}],
              "meets": {"589334": {"MeetName": "SOHI Invite", "EndDate": "2025-05-03T00:00:00"}},
              "resultsTF": [
                {"IDResult": 1, "Result": "26.10a", "FAT": 1, "Place": "3", "Round": "F",
                 "Wind": 1.2, "Division": "Varsity", "SchoolID": 13850, "EventID": 20,
                 "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2025-05-02T00:00:00"},
                {"IDResult": 2, "Result": "DNS", "FAT": 0, "Place": "", "Round": "P",
                 "Division": "Varsity", "SchoolID": 13850, "EventID": 20,
                 "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2025-05-02T00:00:00"}
              ],
              "resultsXC": null
            }"#,
        );
        let rows = bio.results_tf.as_ref().expect("track rows");
        assert_eq!(rows[0].place.as_deref(), Some("3"));
        assert_eq!(rows[0].date().as_deref(), Some("2025-05-02"));
        assert_eq!(rows[1].place, None, "an empty place is no place");
        assert!(bio.results_xc.is_none(), "a null payload is no rows");
        assert_eq!(
            bio.athlete.name(),
            "Natalia Casillas",
            "the canonical name is the published one"
        );

        let xc = payload(
            r#"{
              "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas",
                          "Gender": "F", "SchoolID": 13850},
              "grades": {"13850_2026": 11},
              "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
              "allSeasons": [],
              "meets": {"223703": {"MeetName": "Chandler Invitational", "EndDate": "2025-09-02T00:00:00"}},
              "resultsXC": [
                {"IDResult": 47122798, "Result": "25:31.2", "Place": 68, "Division": "Varsity",
                 "SchoolID": 13850, "MeetID": 223703, "SeasonID": 2025, "Distance": 5000}
              ]
            }"#,
        );
        let xc_rows = xc.results_xc.as_ref().expect("cross-country rows");
        assert_eq!(xc_rows[0].place.as_deref(), Some("68"));
        assert_eq!(xc_rows[0].distance, Some(5000));
    }

    #[test]
    fn the_athleticnet_namespace_is_outside_the_core_scope() {
        let namespace = SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        };
        assert!(
            !namespace.is_core(),
            "the host is not the platform's own source"
        );
        assert!(!is_core_source("athleticnet"));
        assert_eq!(namespace.to_string(), "athleticnet:athlete");
    }
}
