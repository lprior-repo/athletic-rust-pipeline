use super::super::map::{ensure_team, profile_url, Accumulator, PerformanceInput};
use super::read::meet_date;
use super::wire::MeetData;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CompetitionLevel, EventId, Evidence, Gender, Grade, ObservedGrade, SchoolId, SchoolYear,
    SourceAthleteObservation, SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

pub(super) fn meet_url(meet_id: i64) -> String {
    format!("https://www.athletic.net/TrackAndField/meet/{meet_id}/info")
}

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

pub(super) struct AthleteRow<'a> {
    pub(super) provider_id: i64,
    pub(super) school: &'a SchoolId,
    pub(super) name: &'a str,
    pub(super) grade: Grade,
    pub(super) gender: Gender,
    pub(super) school_year: SchoolYear,
    pub(super) sport: Sport,
    pub(super) source_row: &'a str,
}

pub(super) fn athlete(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    row: AthleteRow<'_>,
) -> Option<(AthleteId, SourceIdentity)> {
    let key = format!("{}:{}", row.provider_id, row.school.as_str());
    let observation = ObservedGrade {
        grade: row.grade,
        school_year: row.school_year,
        source: source.clone(),
    };
    let profile = u64::try_from(row.provider_id).ok().map(profile_url);
    let identity = SourceIdentity {
        namespace: SourceNamespace::athletic_net("athlete"),
        id: row.provider_id.to_string(),
        url: profile.clone(),
    };
    let (grad_year, observation, identity) =
        accumulated
            .unsupported
            .admit(observation, identity, |identity| {
                SourceAthleteObservation::new(
                    identity.namespace,
                    identity.id,
                    row.source_row,
                    row.name,
                    observed_on,
                )
                .with_gender(row.gender)
                .with_profile_url(identity.url)
            })?;
    if let Some(athlete) = accumulated.athletes.get_mut(&key) {
        if !athlete.observed_grades.contains(&observation) {
            athlete.observed_grades.push(observation);
        }
        if !athlete.sports.contains(&row.sport) {
            athlete.sports.push(row.sport);
        }
        return Some((athlete.id.clone(), identity));
    }
    let mut athlete = CanonicalAthlete::new(
        row.school,
        row.name,
        grad_year,
        row.gender,
        identity.clone(),
    );
    if let Some(url) = profile {
        athlete.public_profile_urls.push(url);
    }
    athlete.sports.push(row.sport);
    athlete.observed_grades.push(observation);
    athlete
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on));
    let subject = (athlete.id.clone(), identity);
    accumulated.athletes.insert(key, athlete);
    Some(subject)
}

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
                source_athlete: Some(input.source_athlete),
                retained_conflicts: Vec::new(),
            }
        });
}

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
            input
                .division
                .clone()
                .map_or(Default::default(), core::convert::identity),
            input
                .round
                .clone()
                .map_or(Default::default(), core::convert::identity)
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
