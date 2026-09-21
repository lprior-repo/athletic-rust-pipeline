use super::{level_of, school_year_for, Accumulator, ArchiveArtifact, Stats};
use crate::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CompetitionLevel, Evidence, Gender, GradYear, Grade, ObservedGrade, SchoolId, SchoolYear,
    SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
};
use crate::school_index::SchoolIndex;
use crate::sources::result_file::ParsedMeet;
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub(super) fn absorb(
    parsed: &ParsedMeet,
    artifact: &ArchiveArtifact,
    sport: Sport,
    observed_on: &str,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, Option<SchoolId>>,
    stats: &mut Stats,
    accumulator: &mut Accumulator,
) -> usize {
    let level = level_of(&parsed.name);
    // The files never state a timing method. WIAA tournament rounds (regional, sectional, state) are
    // fully automatic per association policy, so the meet's level is the provenance; anything that
    // does not read as a tournament round stays `Unknown` rather than inheriting a "fast" guess.
    let timing = match level {
        CompetitionLevel::State | CompetitionLevel::Sectional | CompetitionLevel::Regional => {
            TimingMethod::Fat
        }
        _ => TimingMethod::Unknown,
    };
    let mut meet = CanonicalMeet::new("WI", parsed.name.clone(), parsed.date.clone(), level);
    meet.end_date = parsed.end_date.clone();
    meet.sports.push(sport);
    meet.source_urls.push(artifact.url.clone());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::Other("wiaa_result_file".to_string()),
        artifact.stem.clone(),
    ));
    let mut meet_evidence = Evidence::parsed(
        SourceRef::new("wiaa_results", Some(artifact.url.clone())),
        observed_on,
    );
    if let Some(timer) = &parsed.timer {
        meet_evidence.note = Some(format!(
            "official WIAA artifact timed by {timer}; label {} ({})",
            artifact.label, artifact.extension
        ));
    } else if parsed.date.len() == 4 {
        meet_evidence.note = Some(format!(
            "date published only as the archive year {}; label {} ({})",
            parsed.date, artifact.label, artifact.extension
        ));
    }
    meet.evidence.push(meet_evidence.clone());

    let school_year = school_year_for(&parsed.date, sport, artifact.year);
    let mut athlete_rows = 0usize;

    for parsed_event in &parsed.events {
        let mut event_entry = CanonicalEvent::new(
            &meet.id,
            parsed_event.kind.clone(),
            parsed_event.gender,
            parsed_event.division.as_deref(),
            parsed_event.round.as_deref(),
        );
        let event_id = event_entry.id.clone();
        event_entry.source_labels.push(SourceEventLabel {
            source: SourceRef::new("wiaa_results", Some(artifact.url.clone())),
            label: parsed_event.label.clone(),
        });
        event_entry.evidence.push(meet_evidence.clone());
        stats.events = stats.events.saturating_add(1);
        accumulator
            .events
            .entry(event_id.as_str().to_string())
            .or_insert(event_entry);

        for (row_index, row) in parsed_event.rows.iter().enumerate() {
            stats.rows = stats.rows.saturating_add(1);
            if row.grade.is_some() {
                stats.rows_with_grade = stats.rows_with_grade.saturating_add(1);
            }
            if row.school.trim().is_empty() {
                stats.rows_without_school = stats.rows_without_school.saturating_add(1);
                continue;
            }
            let school_id = resolved
                .entry(row.school.clone())
                .or_insert_with(|| match index.resolve("WI", &row.school) {
                    Some((id, kind)) => {
                        let resolved_kind = stats.school_resolved.entry(kind.as_str()).or_default();
                        *resolved_kind = resolved_kind.saturating_add(1);
                        Some(id)
                    }
                    None => {
                        let unresolved = stats.unresolved.entry(row.school.clone()).or_default();
                        *unresolved = unresolved.saturating_add(1);
                        None
                    }
                })
                .clone();
            let Some(school_id) = school_id else {
                continue;
            };

            // Individual rows name an athlete; relay rows name a school and list their legs.
            let members: Vec<(Option<u8>, String, Option<Grade>)> = if row.legs.is_empty() {
                vec![(None, row.name.clone(), row.grade)]
            } else {
                row.legs
                    .iter()
                    .map(|leg| (Some(leg.position), leg.name.clone(), leg.grade))
                    .collect()
            };
            if !row.legs.is_empty() {
                stats.relay_legs = stats.relay_legs.saturating_add(row.legs.len());
            }
            let team_id = team_for(
                &mut accumulator.teams,
                &school_id,
                sport,
                parsed_event.gender,
                school_year,
                &meet_evidence,
            );

            for (leg_position, member_name, member_grade) in members {
                let Some(grade) = member_grade else {
                    continue;
                };
                if member_name.trim().is_empty() {
                    continue;
                }
                athlete_rows = athlete_rows.saturating_add(1);
                let grad_year = GradYear::of(grade, school_year);
                let athlete_id = CanonicalAthlete::mint(
                    &school_id,
                    &member_name,
                    grad_year,
                    parsed_event.gender,
                );
                let entry = accumulator
                    .athletes
                    .entry(athlete_id.as_str().to_string())
                    .or_insert_with(|| {
                        let mut athlete = CanonicalAthlete::new(
                            &school_id,
                            &member_name,
                            grad_year,
                            parsed_event.gender,
                        );
                        athlete.sports.push(sport);
                        athlete
                    });
                if !entry.sports.contains(&sport) {
                    entry.sports.push(sport);
                }
                let observation = ObservedGrade {
                    grade,
                    school_year,
                    source: SourceRef::new("wiaa_results", Some(artifact.url.clone())),
                };
                if !entry.observed_grades.contains(&observation) {
                    entry.observed_grades.push(observation);
                }
                if !entry
                    .evidence
                    .iter()
                    .any(|existing| existing == &meet_evidence)
                {
                    entry.evidence.push(meet_evidence.clone());
                }

                let source_key = match leg_position {
                    Some(position) => format!(
                        "{}:{}:{}:{}:leg{}",
                        artifact.stem,
                        parsed_event.label,
                        parsed_event.round.as_deref().unwrap_or("final"),
                        row_index,
                        position
                    ),
                    None => format!(
                        "{}:{}:{}:{}",
                        artifact.stem,
                        parsed_event.label,
                        parsed_event.round.as_deref().unwrap_or("final"),
                        row_index
                    ),
                };
                let performance_id = CanonicalPerformance::mint(
                    &athlete_id,
                    &meet.id,
                    &parsed_event.kind,
                    &meet.date,
                    &source_key,
                );
                let mut evidence = meet_evidence.clone();
                if let Some(position) = leg_position {
                    evidence.note = Some(format!(
                        "relay leg {position} for {}{}; the published mark is the team's",
                        row.school,
                        row.heat
                            .as_deref()
                            .map(|heat| format!(" squad {heat}"))
                            .unwrap_or_default()
                    ));
                }
                accumulator
                    .performances
                    .entry(performance_id.as_str().to_string())
                    .or_insert_with(|| CanonicalPerformance {
                        id: performance_id,
                        athlete: athlete_id,
                        team: team_id.clone(),
                        event: event_id.clone(),
                        meet: meet.id.clone(),
                        date: meet.date.clone(),
                        mark: row.mark.clone(),
                        wind_mps: row.wind_mps,
                        place: row.place,
                        heat: row.heat.clone(),
                        round: parsed_event.round.clone(),
                        timing: Some(timing),
                        observed_grade: Some(grade),
                        evidence: vec![evidence],
                        source_key,
                    });
            }
        }
    }
    accumulator
        .meets
        .entry(meet.id.as_str().to_string())
        .or_insert(meet);
    athlete_rows
}

fn team_for(
    teams: &mut HashMap<String, CanonicalTeam>,
    school: &SchoolId,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    evidence: &Evidence,
) -> crate::model::TeamId {
    let key = format!(
        "{}:{sport:?}:{gender:?}:{}",
        school.as_str(),
        school_year.start_year()
    );
    teams
        .entry(key)
        .or_insert_with(|| {
            let id = CanonicalTeam::mint(school, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
            }
        })
        .id
        .clone()
}
