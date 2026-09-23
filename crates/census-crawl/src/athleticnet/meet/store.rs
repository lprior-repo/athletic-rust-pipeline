//! Minting the canonical entities one meet's rows hang off: the meet itself, its athletes, its
//! events, and the performance rows.

use super::super::map::{ensure_team, profile_url, Accumulator, PerformanceInput};
use super::read::meet_date;
use super::wire::MeetData;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CompetitionLevel, EventId, Evidence, Gender, Grade, ObservedGrade, SchoolId, SchoolYear,
    SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

/// The published meet's own page, the form the sibling AthleticLIVE adapter links it with.
pub(super) fn meet_url(meet_id: i64) -> String {
    format!("https://www.athletic.net/TrackAndField/meet/{meet_id}/info")
}

/// Mint or find the canonical meet, on the same key the athlete-bio path mints it with.
///
/// The level stays [`CompetitionLevel::Unknown`] exactly as the bio path mints it: the bio
/// payload's copy of the same meet publishes no level at all, and one meet must not carry two
/// levels depending on which path read it. (`tfDivisions[].LevelMask` is `4` and the team list's
/// `LevelName` is `"High School"` on this capture; no capture maps a mask value onto the platform's
/// levels, so neither is read as one.)
pub(super) fn meet_row(
    meet: &MeetData,
    state: UsJurisdiction,
    date: &str,
    sport: Sport,
    source: &SourceRef,
    observed_on: &str,
    accumulated: &mut Accumulator,
) -> CanonicalMeet {
    let published = &meet.meet;
    let key = format!("{state}:{}", published.id);
    accumulated
        .meets
        .entry(key)
        .or_insert_with(|| {
            let mut row = CanonicalMeet::new(
                Some(state),
                published.name.clone(),
                date,
                CompetitionLevel::Unknown,
            );
            row.end_date = published
                .end_date
                .as_deref()
                .and_then(meet_date)
                .filter(|end| *end != date)
                .map(str::to_string);
            row.location = published
                .location
                .as_ref()
                .map(|location| location.name.trim().to_string())
                .filter(|name| !name.is_empty());
            row.sports = vec![sport];
            row.source_identities.push(SourceIdentity {
                namespace: SourceNamespace::AthleticNet {
                    kind: "meet".to_string(),
                },
                id: published.id.to_string(),
                url: Some(meet_url(published.id)),
            });
            // `LiveID` is the vendor's own cross-source key for the AthleticLIVE copy of this meet.
            // It is stamped under this adapter's namespace because the id space it uses on
            // AthleticLIVE's side is unverified, so it is evidence rather than a join key.
            if let Some(live_id) = published.live_id {
                row.source_identities.push(SourceIdentity {
                    namespace: SourceNamespace::AthleticNet {
                        kind: "live".to_string(),
                    },
                    id: live_id.to_string(),
                    url: None,
                });
            }
            row.source_urls.push(meet_url(published.id));
            row.evidence
                .push(Evidence::parsed(source.clone(), observed_on));
            row
        })
        .clone()
}

/// One row's athlete identity facts.
pub(super) struct AthleteRow<'a> {
    pub(super) provider_id: i64,
    pub(super) school: &'a SchoolId,
    pub(super) name: &'a str,
    pub(super) grade: Grade,
    pub(super) gender: Gender,
    pub(super) school_year: SchoolYear,
    pub(super) sport: Sport,
}

/// Mint or find the athlete a row names, keyed per (athlete id, school).
///
/// The bio path assigns the identity's whole observed-grade and evidence vectors because one payload
/// describes one athlete; a meet payload names the same athlete from several rows, so the identity
/// is minted on first sight and every further row only adds an observation it does not carry yet.
pub(super) fn athlete(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    row: AthleteRow<'_>,
) -> AthleteId {
    let key = format!("{}:{}", row.provider_id, row.school.as_str());
    let observation = ObservedGrade {
        grade: row.grade,
        school_year: row.school_year,
        source: source.clone(),
    };
    if let Some(athlete) = accumulated.athletes.get_mut(&key) {
        if !athlete.observed_grades.contains(&observation) {
            athlete.observed_grades.push(observation);
        }
        if !athlete.sports.contains(&row.sport) {
            athlete.sports.push(row.sport);
        }
        return athlete.id.clone();
    }
    let mut athlete =
        CanonicalAthlete::new(row.school, row.name, observation.grad_year(), row.gender);
    let profile = u64::try_from(row.provider_id).ok().map(profile_url);
    if let Some(url) = profile.clone() {
        athlete.public_profile_urls.push(url);
    }
    athlete.sports.push(row.sport);
    athlete.observed_grades.push(observation);
    athlete
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on));
    athlete.source_identities.push(SourceIdentity {
        namespace: SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        },
        id: row.provider_id.to_string(),
        url: profile,
    });
    let id = athlete.id.clone();
    accumulated.athletes.insert(key, athlete);
    id
}

/// Store one performance against the event its own division **and round** belong to.
///
/// The bio path's `store_performance` dedupes events on (meet, kind, gender, division) — a key that
/// omits the round — so the meet path does not reuse it: a meet payload publishes prelims and finals
/// of the same event, and the domain's event identity includes the round (this capture carries 4
/// such pairs, so 49 event identities against 45 round-less keys). The team mint *is* shared, and the
/// performance row is assembled exactly as the bio path assembles it.
pub(super) fn store(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: PerformanceInput<'_>,
    note: Option<String>,
) {
    let team = ensure_team(accumulated, source, observed_on, &input);
    let event = event_of(accumulated, source, observed_on, &input);
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
        .or_insert_with(|| {
            let mut evidence = Evidence::parsed(source.clone(), observed_on);
            evidence.note = note;
            CanonicalPerformance {
                id: performance_id,
                athlete: input.athlete.clone(),
                team,
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
                evidence: vec![evidence],
                source_key: input.source_key,
                source_athlete: None,
                retained_conflicts: Vec::new(),
            }
        });
}

/// The event a performance belongs to: the bio path's mint, deduped **with** the round.
fn event_of(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: &PerformanceInput<'_>,
) -> EventId {
    accumulated
        .events
        .entry(format!(
            "{}:{:?}:{:?}:{}:{}",
            input.meet.id.as_str(),
            input.kind,
            input.gender,
            input.division.clone().unwrap_or_default(),
            input.round.clone().unwrap_or_default()
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
        .clone()
}
