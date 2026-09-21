//! AthleticLIVE athlete-index adapter.
//!
//! AthleticLIVE is the white-label live-results platform many Midwest timers run. Its public
//! Elasticsearch endpoint `search.athletic.live/athlete_list/_search` indexes one document per
//! athlete-entry at a meet, and those documents carry, per row:
//!
//! - the competitor's name, sex, and **grade** (`y`),
//! - the **Athletic.net athlete id** (`ani`) when the operator linked the meet,
//! - the school name plus AthleticLIVE team id (`t.i`) and **Athletic.net team id** (`t.ani`).
//!
//! That makes this the second independent athlete source in the census (MileSplit rosters are the
//! first) and, more importantly, a source of *Athletic.net profile seeds*: for every row with an
//! `ani`, the profile URL is deterministic
//! (`https://www.athletic.net/athlete/{id}/track-and-field`), so no Athletic.net enumeration or
//! search is required to acquire that athlete's history.
//!
//! Query shape (verified 2026-09-20 against meets 73566 and 75742):
//!
//! ```text
//! POST https://search.athletic.live/athlete_list/_search
//! {"size":2000,"from":0,
//!  "query":{"bool":{"filter":[{"terms":{"mi":[<AthleticLIVE meet ids>]}},
//!                             {"terms":{"y":["11","12","JR","SR"]}}]}},
//!  "_source":["i","n","y","g","mi","ani","t"]}
//! ```
//!
//! `y` is a keyword and accepts the numeric encoding (`"11"`) used by track meets; letter encodings
//! (`JR`/`SR`) appear on some cross-country meets and are filtered for as well. Grade is interpreted
//! against the meet date's school year, never against "today": grade 11 at a 2025-26 meet is class of
//! 2027, grade 12 at a 2026-27 meet is also class of 2027, and both are retained as observations.
//!
//! Elasticsearch caps `from + size` at 10,000, so meet batches are split when a batch exceeds the
//! window, and pagination restarts at the top of each split.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalSchool, CanonicalTeam, Evidence, Gender, GradYear,
    Grade, ObservedGrade, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;

/// Elasticsearch result window: `from + size` may not exceed this.
const RESULT_WINDOW: usize = 10_000;
/// Rows per page.
const PAGE_SIZE: usize = 2_000;
/// Meet ids per query.
const MEETS_PER_BATCH: usize = 40;

const ENDPOINT: &str = "https://search.athletic.live/athlete_list/_search";

/// Adapter options (uniform across provider adapters).
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to meets in these state codes; empty = every state present in the meet log.
    pub states: Vec<String>,
    pub school_names: Vec<String>,
}

impl Options {
    pub fn for_states(states: Vec<String>, observed_on: impl Into<String>) -> Self {
        Self {
            states,
            observed_on: observed_on.into(),
            ..Default::default()
        }
    }
}

/// One `athlete_list` document.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AthleteHit {
    /// Row id (per meet-entry; NOT a person key).
    #[serde(default)]
    pub i: Option<Value>,
    #[serde(default)]
    pub n: Option<String>,
    /// Grade token: `"9".."12"`, `FR|SO|JR|SR`, occasionally blank.
    #[serde(default)]
    pub y: Option<Value>,
    /// `"Male"` / `"Female"`.
    #[serde(default)]
    pub g: Option<String>,
    /// AthleticLIVE meet id.
    #[serde(default)]
    pub mi: Option<Value>,
    /// Athletic.net athlete id.
    #[serde(default)]
    pub ani: Option<Value>,
    #[serde(default)]
    pub t: Option<HitTeam>,
}

/// Team object embedded in an athlete row.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HitTeam {
    /// AthleticLIVE team id.
    #[serde(default)]
    pub i: Option<Value>,
    #[serde(default)]
    pub n: Option<String>,
    #[serde(default)]
    pub f: Option<String>,
    #[serde(default)]
    pub ab: Option<String>,
    /// Athletic.net team id.
    #[serde(default)]
    pub ani: Option<Value>,
    /// Cross-country marker (`1` on XC meets).
    #[serde(default)]
    pub xc: Option<Value>,
}

impl HitTeam {
    /// School name as published: long name preferred, then short name.
    pub fn school_name(&self) -> Option<&str> {
        self.n
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| self.f.as_deref().filter(|v| !v.trim().is_empty()))
    }
}

/// Numeric value from a JSON number or numeric string.
fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.trim().parse::<u64>().ok(),
        _ => None,
    }
}

impl AthleteHit {
    pub fn athletic_net_athlete_id(&self) -> Option<u64> {
        self.ani.as_ref().and_then(as_u64)
    }

    pub fn athleticlive_row_id(&self) -> Option<u64> {
        self.i.as_ref().and_then(as_u64)
    }

    pub fn meet_id(&self) -> Option<u64> {
        self.mi.as_ref().and_then(as_u64)
    }
}

impl HitTeam {
    pub fn athletic_net_team_id(&self) -> Option<u64> {
        self.ani.as_ref().and_then(as_u64)
    }

    pub fn athleticlive_team_id(&self) -> Option<u64> {
        self.i.as_ref().and_then(as_u64)
    }

    pub fn is_cross_country(&self) -> bool {
        match &self.xc {
            Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(false),
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s.trim() != "0" && !s.trim().is_empty(),
            _ => false,
        }
    }
}

/// Parse a grade token from the athlete index.
///
/// Returns `None` for blanks and for values outside the four high-school grades; grade 8 and below
/// are out of contract for this census.
pub fn grade_from_token(token: &str) -> Option<Grade> {
    let cleaned = token.trim().trim_start_matches('0').to_ascii_uppercase();
    let by_name = match cleaned.as_str() {
        "FR" | "FRESHMAN" => Some(9),
        "SO" | "SOPHOMORE" => Some(10),
        "JR" | "JUNIOR" => Some(11),
        "SR" | "SENIOR" => Some(12),
        _ => None,
    };
    if let Some(grade) = by_name {
        return Grade::new(grade);
    }
    cleaned.parse::<u8>().ok().and_then(Grade::new)
}

/// Grade year for a row's meet date. Unparseable dates fall back to the caller's school year.
pub fn school_year_for_date(date: &str, fallback: SchoolYear) -> SchoolYear {
    let year = date.get(..4).and_then(|y| y.parse::<i16>().ok());
    let month = date.get(5..7).and_then(|m| m.parse::<u8>().ok());
    match (year, month) {
        (Some(year), Some(month)) if (1..=12).contains(&month) => {
            SchoolYear::containing(year, month)
        }
        _ => fallback,
    }
}

/// Gender token as published by AthleticLIVE.
pub fn gender_from_token(token: &str) -> Gender {
    match token.trim().to_ascii_lowercase().as_str() {
        "male" | "m" | "boys" | "boy" => Gender::Boys,
        "female" | "f" | "girls" | "girl" => Gender::Girls,
        _ => Gender::Unknown,
    }
}

/// Sport of a row: the team's cross-country marker wins, otherwise the meet name decides, and the
/// meet month is the last resort (indoor meets run Dec-Mar, outdoor Apr-Jul).
pub fn sport_for(team_is_xc: bool, meet_name: &str, meet_date: &str) -> Sport {
    if team_is_xc {
        return Sport::CrossCountry;
    }
    let lowered = meet_name.to_ascii_lowercase();
    if lowered.contains("cross country") || lowered.contains("xc ") || lowered.ends_with(" xc") {
        return Sport::CrossCountry;
    }
    if lowered.contains("indoor") || lowered.contains("mits") {
        return Sport::IndoorTrack;
    }
    match meet_date.get(5..7).and_then(|m| m.parse::<u8>().ok()) {
        Some(month @ (12 | 1 | 2 | 3)) => {
            let _ = month;
            Sport::IndoorTrack
        }
        Some(_) => Sport::OutdoorTrack,
        None => Sport::OutdoorTrack,
    }
}

/// A canonical meet plus the timer identity this adapter queries it by.
#[derive(Debug, Clone)]
pub struct MeetTarget {
    pub athleticlive_meet_id: u64,
    pub meet_id: String,
    pub tenant: String,
    pub name: String,
    pub state: String,
    pub date: String,
}

/// Meets the adapter will query, plus the count it refused to query.
#[derive(Debug, Clone, Default)]
pub struct MeetSelection {
    pub targets: Vec<MeetTarget>,
    /// Timer-published meets dropped because their published date cannot be trusted.
    ///
    /// Tenant meet indexes carry placeholder rows (`date` in the 2220s). An athlete's graduating
    /// class is derived from the meet date, so an implausible date would mint an implausible
    /// class; those meets are skipped rather than guessed at.
    pub skipped_implausible: usize,
}

/// The window a meet date must fall in to be usable. Same convention as the meet-index harvest.
const MEET_YEAR_MIN: i16 = 2015;
const MEET_YEAR_MAX: i16 = 2030;

fn plausible_meet_year(date: &str) -> bool {
    match date.get(..4).and_then(|year| year.parse::<i16>().ok()) {
        Some(year) => (MEET_YEAR_MIN..=MEET_YEAR_MAX).contains(&year),
        None => false,
    }
}

/// Select meets that a timer published, keyed by their AthleticLIVE meet id.
///
/// Meets are deduplicated by canonical id: several tenants publishing one meet collapse to the first
/// target, because the athlete rows are keyed by the AthleticLIVE meet id and duplicate ids would
/// multiply requests.
pub fn meet_targets(meets: &[CanonicalMeet], states: &[String]) -> MeetSelection {
    let wanted: BTreeSet<String> = states
        .iter()
        .map(|state| state.trim().to_ascii_uppercase())
        .collect();
    let mut seen: HashMap<u64, MeetTarget> = HashMap::new();
    let mut skipped_implausible = 0usize;
    for meet in meets {
        if !wanted.is_empty() && !wanted.contains(&meet.state.to_ascii_uppercase()) {
            continue;
        }
        if !plausible_meet_year(&meet.date) {
            skipped_implausible += 1;
            continue;
        }
        for identity in &meet.source_identities {
            let SourceNamespace::TimerMeet { provider } = &identity.namespace else {
                continue;
            };
            let Ok(athleticlive_meet_id) = identity.id.parse::<u64>() else {
                continue;
            };
            seen.entry(athleticlive_meet_id)
                .or_insert_with(|| MeetTarget {
                    athleticlive_meet_id,
                    meet_id: meet.id.as_str().to_string(),
                    tenant: provider.clone(),
                    name: meet.name.clone(),
                    state: meet.state.clone(),
                    date: meet.date.clone(),
                });
        }
    }
    let mut targets: Vec<MeetTarget> = seen.into_values().collect();
    targets.sort_by_key(|target| {
        (
            target.state.clone(),
            target.date.clone(),
            target.athleticlive_meet_id,
        )
    });
    MeetSelection {
        targets,
        skipped_implausible,
    }
}

/// Build the Elasticsearch query for a batch of meet ids.
pub fn batch_query(meet_ids: &[u64], from: usize) -> Value {
    json!({
        "size": PAGE_SIZE,
        "from": from,
        "track_total_hits": true,
        "query": { "bool": { "filter": [
            { "terms": { "mi": meet_ids } },
            { "terms": { "y": ["11", "12", "JR", "SR", "Jr", "Sr"] } }
        ] } },
        "_source": ["i", "n", "y", "g", "mi", "ani", "t"]
    })
}

/// Entities minted from one batch of athlete rows.
#[derive(Debug, Default)]
pub struct BatchEntities {
    pub schools: Vec<CanonicalSchool>,
    pub teams: Vec<CanonicalTeam>,
    pub athletes: Vec<CanonicalAthlete>,
    pub rows: usize,
    pub rows_with_grade: usize,
    pub rows_with_athlete_id: usize,
    pub rows_with_team_id: usize,
    pub rows_without_school: usize,
}

/// Canonical entities for a batch of athlete rows, deduplicated by canonical id.
///
/// The athlete key is (school, name, grad year, gender): two meets that disagree on nothing produce
/// one athlete, and the second meet's grade observation is appended rather than replacing the first.
pub fn build_entities(
    hits: &[AthleteHit],
    targets: &HashMap<u64, &MeetTarget>,
    observed_on: &str,
    fallback_year: SchoolYear,
) -> BatchEntities {
    let mut out = BatchEntities::default();
    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    let mut teams: BTreeMap<(String, String, String, SchoolYear), CanonicalTeam> = BTreeMap::new();
    let mut athletes: BTreeMap<String, CanonicalAthlete> = BTreeMap::new();

    for hit in hits {
        out.rows += 1;
        let Some(meet_id) = hit.meet_id() else {
            continue;
        };
        let Some(target) = targets.get(&meet_id) else {
            continue;
        };
        let Some(name) = hit.n.as_deref().map(str::trim).filter(|n| !n.is_empty()) else {
            continue;
        };
        let Some(team) = hit.t.as_ref().filter(|t| t.school_name().is_some()) else {
            out.rows_without_school += 1;
            continue;
        };
        let school_name = team.school_name().unwrap_or_default().trim();
        let source_url = format!("{ENDPOINT} (mi={meet_id})");
        let evidence = Evidence::parsed(
            SourceRef::new("athleticlive_athletes", Some(source_url.clone())),
            observed_on,
        );

        let gender = hit
            .g
            .as_deref()
            .map(gender_from_token)
            .unwrap_or(Gender::Unknown);
        let grade = hit.y.as_ref().and_then(|value| match value {
            Value::String(s) => grade_from_token(s),
            Value::Number(n) => grade_from_token(&n.to_string()),
            _ => None,
        });
        let sport = sport_for(team.is_cross_country(), &target.name, &target.date);
        let school_year = school_year_for_date(&target.date, fallback_year);

        // School: canonical id from state + normalized name, so a MileSplit school of the same name
        // and state mints the same school.
        let (mut school, school_id) = CanonicalSchool::new(
            &target.state,
            school_name,
            crate::model::normalize_name(school_name),
        );
        if !school.evidence.iter().any(|e| e == &evidence) {
            school.evidence.push(evidence.clone());
        }
        schools
            .entry(school_id.as_str().to_string())
            .or_insert(school);

        // Team: one per (school, sport, gender, school year).
        // One team per (school, sport, gender, school year) - the same key MileSplit uses, so
        // both sources mint the same team id for the same team.
        let team_key = (
            school_id.as_str().to_string(),
            format!("{sport:?}"),
            format!("{gender:?}"),
            school_year,
        );
        let team_entry = teams.entry(team_key).or_insert_with(|| {
            let id = CanonicalTeam::mint(&school_id, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school_id.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
            }
        });
        if let Some(timer_team_id) = team.athleticlive_team_id() {
            let identity = SourceIdentity::new(
                SourceNamespace::TimerTeam {
                    provider: target.tenant.clone(),
                },
                timer_team_id.to_string(),
            );
            if !team_entry.source_identities.contains(&identity) {
                team_entry.source_identities.push(identity);
            }
        }
        if let Some(an_team_id) = team.athletic_net_team_id() {
            out.rows_with_team_id += 1;
            let identity = SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "team".to_string(),
                },
                an_team_id.to_string(),
            );
            if !team_entry.source_identities.contains(&identity) {
                team_entry.source_identities.push(identity);
            }
        }

        // Grade is required: an athlete entity is minted from (school, name, grad year, gender).
        let Some(grade) = grade else { continue };
        out.rows_with_grade += 1;
        let grad_year = GradYear::of(grade, school_year);
        let athlete_id = CanonicalAthlete::mint(&school_id, name, grad_year, gender);
        let entry = athletes
            .entry(athlete_id.as_str().to_string())
            .or_insert_with(|| {
                let mut athlete = CanonicalAthlete::new(&school_id, name, grad_year, gender);
                athlete.sports.push(sport);
                athlete.evidence.push(evidence.clone());
                athlete
            });
        if !entry.sports.contains(&sport) {
            entry.sports.push(sport);
        }
        let observation = ObservedGrade {
            grade,
            school_year,
            source: SourceRef::new("athleticlive_athletes", Some(source_url.clone())),
        };
        if !entry.observed_grades.contains(&observation) {
            entry.observed_grades.push(observation);
        }
        if let Some(an_athlete_id) = hit.athletic_net_athlete_id() {
            out.rows_with_athlete_id += 1;
            let identity = SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "athlete".to_string(),
                },
                an_athlete_id.to_string(),
            );
            if !entry.source_identities.contains(&identity) {
                entry.source_identities.push(identity);
            }
            // Athletic.net profile URLs are deterministic from the athlete id (research report 02).
            let profile_url =
                format!("https://www.athletic.net/athlete/{an_athlete_id}/track-and-field");
            if !entry.public_profile_urls.contains(&profile_url) {
                entry.public_profile_urls.push(profile_url);
            }
        }
    }

    out.schools = schools.into_values().collect();
    out.teams = teams.into_values().collect();
    out.athletes = athletes.into_values().collect();
    out
}

/// Collect athlete rows for every timer-published meet and emit canonical entities.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let meets: Vec<CanonicalMeet> = ctx.store.scan(Table::Meets)?;
    if meets.is_empty() {
        bail!("no meets in the store: run the `athleticlive` adapter first");
    }
    let selection = meet_targets(&meets, &options.states);
    let targets = selection.targets;
    if targets.is_empty() {
        bail!("no timer-published meets matched the requested states");
    }
    let by_id: HashMap<u64, &MeetTarget> = targets
        .iter()
        .map(|t| (t.athleticlive_meet_id, t))
        .collect();

    let mut report = AdapterReport::new("athleticlive_athletes", "athletes");
    let journal = ctx.store.journal_keys("athleticlive_rosters")?;
    let pending: Vec<&MeetTarget> = targets
        .iter()
        .filter(|target| !journal.contains(&target.athleticlive_meet_id.to_string()))
        .collect();
    report.note(format!(
        "meets in store: {}; timer-published after state filter: {}; skipped (implausible date): {}; already journaled: {}; pending: {}",
        meets.len(),
        targets.len(),
        selection.skipped_implausible,
        journal.len(),
        pending.len()
    ));

    // FIFO: batches are consumed oldest-meet-first, so a capped run covers settled meets with
    // results rather than the tail of future-dated entries.
    let mut queue: VecDeque<Vec<&MeetTarget>> = pending
        .chunks(MEETS_PER_BATCH)
        .map(|chunk| chunk.to_vec())
        .collect();
    let mut stats = BatchStats::default();
    let (requests_before, cache_before) = stats_of(ctx).await;

    while let Some(batch) = queue.pop_front() {
        if let Some(limit) = options.limit {
            if stats.meets >= limit {
                break;
            }
        }
        let ids: Vec<u64> = batch.iter().map(|t| t.athleticlive_meet_id).collect();
        let mut from = 0usize;
        let mut hits: Vec<AthleteHit> = Vec::new();
        let mut total = 0usize;
        loop {
            let body = batch_query(&ids, from);
            let outcome = ctx
                .fetcher
                .post_json(
                    ENDPOINT,
                    &body,
                    &crate::net::FetchOptions {
                        refresh: options.refresh,
                        allow_not_found: false,
                        headers: vec![("accept".to_string(), "application/json".to_string())],
                    },
                )
                .await
                .context("querying athleticlive athlete_list")?;
            if outcome.status != 200 {
                report.errors += 1;
                report.note(format!(
                    "batch of {} meets returned HTTP {} at offset {from}",
                    ids.len(),
                    outcome.status
                ));
                break;
            }
            let parsed: Value = outcome.json().context("parsing athlete_list response")?;
            let page_total = parsed
                .pointer("/hits/total/value")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if from == 0 {
                total = page_total;
            }
            let sources: Vec<Value> = parsed
                .pointer("/hits/hits")
                .and_then(Value::as_array)
                .map(|hits| {
                    hits.iter()
                        .map(|hit| hit.get("_source").cloned().unwrap_or(Value::Null))
                        .collect()
                })
                .unwrap_or_default();
            let page: Vec<AthleteHit> = serde_json::from_value(Value::Array(sources))
                .context("decoding athlete_list hits")?;
            let fetched = page.len();
            hits.extend(page);
            from += fetched;
            if fetched == 0 || from >= total || from + PAGE_SIZE > RESULT_WINDOW {
                break;
            }
        }

        // A batch whose total exceeds the Elasticsearch result window must be split: the missing
        // rows are not recoverable by paging past 10,000.
        if total > RESULT_WINDOW {
            if batch.len() > 1 {
                let (left, right) = batch.split_at(batch.len() / 2);
                queue.push_front(right.to_vec());
                queue.push_front(left.to_vec());
                stats.splits += 1;
                report.note(format!(
                    "split a {}-meet batch ({} rows exceeds the {}-row result window)",
                    batch.len(),
                    total,
                    RESULT_WINDOW
                ));
                continue;
            }
            report.note(format!(
                "meet {} alone has {} rows: only {} were retrievable in one result window",
                batch[0].athleticlive_meet_id, total, RESULT_WINDOW
            ));
        }

        if hits.is_empty() {
            for target in &batch {
                ctx.store.journal_done(
                    "athleticlive_rosters",
                    &target.athleticlive_meet_id.to_string(),
                    &json!({ "meet": target.name, "rows": 0 }),
                )?;
            }
            stats.meets += batch.len();
            continue;
        }

        let entities = build_entities(&hits, &by_id, &options.observed_on, ctx.school_year);
        ctx.store.append_many(Table::Schools, &entities.schools)?;
        ctx.store.append_many(Table::Teams, &entities.teams)?;
        ctx.store.append_many(Table::Athletes, &entities.athletes)?;
        for target in &batch {
            ctx.store.journal_done(
                "athleticlive_rosters",
                &target.athleticlive_meet_id.to_string(),
                &json!({ "meet": target.name, "batch_rows": hits.len() }),
            )?;
        }
        stats.meets += batch.len();
        stats.rows += entities.rows;
        stats.athletes += entities.athletes.len();
        stats.schools += entities.schools.len();
        stats.teams += entities.teams.len();
        stats.rows_with_grade += entities.rows_with_grade;
        stats.rows_with_athlete_id += entities.rows_with_athlete_id;
        stats.rows_with_team_id += entities.rows_with_team_id;
        stats.rows_without_school += entities.rows_without_school;
    }

    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = stats.athletes as u64;
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.note(format!(
        "meets processed: {}; athlete rows: {}; canonical athletes written: {}",
        stats.meets, stats.rows, stats.athletes
    ));
    report.note(format!(
        "schools written: {}; teams written: {}; batch splits: {}",
        stats.schools, stats.teams, stats.splits
    ));
    report.note(format!(
        "rows with a grade: {}/{}, with an Athletic.net athlete id: {}, with an Athletic.net team id: {}, without a school name: {}",
        stats.rows_with_grade, stats.rows, stats.rows_with_athlete_id, stats.rows_with_team_id, stats.rows_without_school
    ));
    if !options.states.is_empty() {
        report.note(format!("state filter: {}", options.states.join(",")));
    }
    Ok(report)
}

#[derive(Debug, Default)]
struct BatchStats {
    meets: usize,
    rows: usize,
    athletes: usize,
    schools: usize,
    teams: usize,
    rows_with_grade: usize,
    rows_with_athlete_id: usize,
    rows_with_team_id: usize,
    rows_without_school: usize,
    splits: usize,
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        include_str!("../../tests/fixtures/athleticlive_athletes/athlete-list-sample.json");

    fn targets_for() -> Vec<MeetTarget> {
        let mut meet = CanonicalMeet::new(
            "KS",
            "Abilene Invitational",
            "2025-04-25",
            crate::model::CompetitionLevel::Invitational,
        );
        meet.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "reddirt".into(),
            },
            "73566".to_string(),
        ));
        let meets = vec![meet];
        meet_targets(&meets, &["KS".to_string()]).targets
    }

    fn parsed_hits() -> Vec<AthleteHit> {
        let value: Value = serde_json::from_str(SAMPLE).expect("fixture is JSON");
        let sources: Vec<Value> = value["hits"]["hits"]
            .as_array()
            .expect("fixture has hits")
            .iter()
            .map(|hit| hit["_source"].clone())
            .collect();
        serde_json::from_value(Value::Array(sources)).expect("hits decode")
    }

    #[test]
    fn grade_tokens_cover_numeric_and_letter_encodings() {
        assert_eq!(grade_from_token("11").map(Grade::get), Some(11));
        assert_eq!(grade_from_token(" 12 ").map(Grade::get), Some(12));
        assert_eq!(grade_from_token("JR").map(Grade::get), Some(11));
        assert_eq!(grade_from_token("Sr").map(Grade::get), Some(12));
        assert_eq!(grade_from_token("FR").map(Grade::get), Some(9));
        assert_eq!(grade_from_token(""), None);
        assert_eq!(
            grade_from_token("5"),
            None,
            "middle-school grades are out of contract"
        );
        assert_eq!(grade_from_token("null"), None);
    }

    #[test]
    fn grade_is_interpreted_against_the_meet_school_year() {
        let fallback = SchoolYear(2026);
        // Grade 11 in a 2025-26 meet is class of 2027.
        let spring_2026 = school_year_for_date("2026-04-25", fallback);
        assert_eq!(GradYear::of(Grade::new(11).unwrap(), spring_2026).0, 2027);
        // Grade 12 in a 2026-27 meet is also class of 2027.
        let fall_2026 = school_year_for_date("2026-09-12", fallback);
        assert_eq!(GradYear::of(Grade::new(12).unwrap(), fall_2026).0, 2027);
        // A 2025 XC meet (2025-26) with grade 11 is class of 2027 too.
        let fall_2025 = school_year_for_date("2025-10-04", fallback);
        assert_eq!(GradYear::of(Grade::new(11).unwrap(), fall_2025).0, 2027);
        // Bad dates fall back rather than panicking.
        assert_eq!(school_year_for_date("", fallback).0, 2026);
        assert_eq!(school_year_for_date("garbage", fallback).0, 2026);
    }

    #[test]
    fn rows_become_canonical_entities_with_athletic_net_seeds() {
        let hits = parsed_hits();
        assert!(!hits.is_empty(), "fixture must contain rows");
        let targets = targets_for();
        let by_id: HashMap<u64, &MeetTarget> = targets
            .iter()
            .map(|t| (t.athleticlive_meet_id, t))
            .collect();
        let entities = build_entities(&hits, &by_id, "2026-09-20", SchoolYear(2026));

        assert_eq!(entities.rows, hits.len());
        assert!(
            entities.rows_with_grade > 0,
            "the fixture carries graded rows for this meet"
        );
        assert!(!entities.athletes.is_empty(), "graded rows mint athletes");
        for athlete in &entities.athletes {
            // Fixture meet is 2025-04 (school year 2024-25): grade 9-12 maps to classes 2025-2028.
            assert!(
                (2024..=2031).contains(&athlete.grad_year.0),
                "grad year {} is outside the plausible window for this meet",
                athlete.grad_year.0
            );
            assert!(
                !athlete.public_profile_urls.is_empty(),
                "rows carrying an Athletic.net athlete id must expose the deterministic profile URL"
            );
            for url in &athlete.public_profile_urls {
                assert!(url.starts_with("https://www.athletic.net/athlete/"));
                assert!(url.ends_with("/track-and-field"));
            }
            assert!(
                athlete
                    .observed_grades
                    .iter()
                    .all(|g| g.source.id == "athleticlive_athletes"),
                "grade observations keep their own source"
            );
        }
    }

    #[test]
    fn one_athlete_seen_at_two_meets_stays_one_athlete() {
        let base = parsed_hits();
        let Some(first) = base.first().cloned() else {
            return;
        };
        let mut second = first.clone();
        second.mi = Some(json!(999_999));
        let first_meet_id = first.meet_id().unwrap();
        let hits = vec![first, second];
        let meet_a = MeetTarget {
            athleticlive_meet_id: first_meet_id,
            meet_id: "meet_a".into(),
            tenant: "reddirt".into(),
            name: "Abilene Invitational".into(),
            state: "KS".into(),
            date: "2025-04-25".into(),
        };
        let meet_b = MeetTarget {
            athleticlive_meet_id: 999_999,
            meet_id: "meet_b".into(),
            tenant: "reddirt".into(),
            name: "Abilene Invitational".into(),
            state: "KS".into(),
            date: "2025-04-25".into(),
        };
        let by_id: HashMap<u64, &MeetTarget> =
            [(meet_a.athleticlive_meet_id, &meet_a), (999_999, &meet_b)]
                .into_iter()
                .collect();
        let entities = build_entities(&hits, &by_id, "2026-09-20", SchoolYear(2026));
        assert_eq!(entities.rows, 2);
        assert_eq!(
            entities.athletes.len(),
            1,
            "same school+name+grade+gender across meets is one canonical athlete"
        );
        assert_eq!(entities.teams.len(), 1, "and one canonical team");
    }

    #[test]
    fn meet_targets_deduplicate_by_athleticlive_id_and_respect_state_filter() {
        let mut meet = CanonicalMeet::new(
            "KS",
            "Abilene Invitational",
            "2025-04-25",
            crate::model::CompetitionLevel::Invitational,
        );
        meet.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "reddirt".into(),
            },
            "73566".to_string(),
        ));
        let mut duplicate = meet.clone();
        duplicate.source_identities = vec![SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "athleticlive".into(),
            },
            "73566".to_string(),
        )];
        let other_state = CanonicalMeet::new(
            "SD",
            "Dakota XC",
            "2025-10-04",
            crate::model::CompetitionLevel::Invitational,
        );
        let mut other_state = other_state;
        other_state.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "dakota".into(),
            },
            "75742".to_string(),
        ));
        let targets = meet_targets(&[meet, duplicate, other_state], &["KS".to_string()]).targets;
        assert_eq!(
            targets.len(),
            1,
            "duplicate ids and other states are excluded"
        );
        assert_eq!(targets[0].athleticlive_meet_id, 73566);
    }

    #[test]
    fn implausible_meet_dates_are_skipped_and_counted() {
        // The tenant meet index carries placeholder rows dated in the 2220s. A grade interpreted
        // against such a date would mint a class of 2223, so the meet is refused, not guessed at.
        let mut placeholder = CanonicalMeet::new(
            "KS",
            "Sample Meet",
            "2222-04-15",
            crate::model::CompetitionLevel::Invitational,
        );
        placeholder.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "athleticlive".into(),
            },
            "31939".to_string(),
        ));
        let mut real = CanonicalMeet::new(
            "KS",
            "Abilene Invitational",
            "2025-04-25",
            crate::model::CompetitionLevel::Invitational,
        );
        real.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "reddirt".into(),
            },
            "73566".to_string(),
        ));
        let selection = meet_targets(&[placeholder, real], &["KS".to_string()]);
        assert_eq!(
            selection.targets.len(),
            1,
            "only the plausible meet survives"
        );
        assert_eq!(selection.targets[0].athleticlive_meet_id, 73566);
        assert_eq!(
            selection.skipped_implausible, 1,
            "the refusal is counted, not silent"
        );
    }

    #[test]
    fn query_filters_grades_server_side() {
        let query = batch_query(&[73566, 75742], 2000);
        assert_eq!(query["from"], 2000);
        let filters = &query["query"]["bool"]["filter"];
        assert_eq!(filters[0]["terms"]["mi"][0], 73566);
        let grades = filters[1]["terms"]["y"].as_array().expect("grade terms");
        assert!(grades.iter().any(|v| v == "11"));
        assert!(grades.iter().any(|v| v == "JR"));
    }

    #[test]
    fn empty_and_malformed_hits_yield_nothing_instead_of_panicking() {
        let targets = targets_for();
        let by_id: HashMap<u64, &MeetTarget> = targets
            .iter()
            .map(|t| (t.athleticlive_meet_id, t))
            .collect();
        let entities = build_entities(&[], &by_id, "2026-09-20", SchoolYear(2026));
        assert_eq!(entities.rows, 0);
        assert!(entities.athletes.is_empty());

        let nameless = AthleteHit {
            y: Some(Value::String("11".into())),
            ..Default::default()
        };
        let entities = build_entities(&[nameless], &by_id, "2026-09-20", SchoolYear(2026));
        assert_eq!(entities.rows, 1);
        assert!(
            entities.athletes.is_empty(),
            "a row without a name mints nothing"
        );

        let mut missing_school = parsed_hits().into_iter().next().unwrap();
        missing_school.t = None;
        let entities = build_entities(&[missing_school], &by_id, "2026-09-20", SchoolYear(2026));
        assert_eq!(entities.rows_without_school, 1);
        assert!(entities.athletes.is_empty());
    }
}
