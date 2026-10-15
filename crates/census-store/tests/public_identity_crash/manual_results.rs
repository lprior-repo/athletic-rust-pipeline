use census_domain::model::{
    AthleteCandidateId, AthleteId, CanonicalPerformance, EventId, Mark, MeetId,
    PerformanceIdentity, PerformanceResult, RelayMember, RelayResult, TeamId,
};
use census_store::{Store, Table};

use super::TestResult;

pub(super) fn install(store: &Store, candidates: &[AthleteCandidateId; 3]) -> TestResult {
    for (index, candidate) in candidates.iter().enumerate() {
        let athlete = candidate.cast();
        let label = index.to_string();
        store.append(Table::Performances, &performance(&athlete, &label)?)?;
        store.append(Table::RelayResults, &relay(&athlete, &label))?;
    }
    Ok(())
}

pub(super) fn expected_keys(candidates: &[AthleteCandidateId; 3]) -> TestResult<Vec<String>> {
    let mut keys = Vec::new();
    for (index, candidate) in candidates.iter().take(2).enumerate() {
        let athlete = candidate.cast();
        let label = index.to_string();
        keys.push(format!("athletes:{candidate}"));
        keys.push(format!(
            "performances:{}",
            performance(&athlete, &label)?.id
        ));
        keys.push(format!("relay_results:{}", relay(&athlete, &label).id));
    }
    keys.sort();
    Ok(keys)
}

fn performance(athlete: &AthleteId, label: &str) -> TestResult<CanonicalPerformance> {
    let meet = MeetId::mint("meet", &["manual-crash", label]);
    let event = EventId::mint("event", &["manual-crash-100m", label]);
    let team = TeamId::mint("team", &["manual-crash-team", label]);
    Ok(CanonicalPerformance::new(
        PerformanceIdentity {
            athlete,
            event: &event,
            meet: &meet,
            date: "2026-10-15",
            source_key: label,
        },
        PerformanceResult {
            team: &team,
            mark: Mark::Raw("12.19".into()),
            wind_mps: None,
            place: Some(1),
            adjustment: None,
        },
    )?)
}

fn relay(athlete: &AthleteId, label: &str) -> RelayResult {
    RelayResult::new(
        &TeamId::mint("team", &["manual-crash-relay-team", label]),
        &EventId::mint("event", &["manual-crash-4x100", label]),
        &MeetId::mint("meet", &["manual-crash", label]),
        Mark::Raw("49.19".into()),
        label,
    )
    .with_member(RelayMember::new(1, "Synthetic crash runner").with_athlete(athlete.clone()))
}
