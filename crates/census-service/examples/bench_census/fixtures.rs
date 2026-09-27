
use anyhow::{Context, Result};
use census_domain::model::{
    CanonicalMeet, CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, Grade, Id,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport, TeamId,
};
use census_domain::UsJurisdiction;

use super::lcg::Lcg;

pub(super) const SOURCE_ID: &str = "mshsl_results";
pub(super) const MEET_DATE: &str = "2026-05-02";

pub(super) fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

pub(super) fn grade(value: u8) -> Result<Grade> {
    Grade::new(value).context("grade must be between 9 and 12")
}

pub(super) fn season() -> Result<SchoolYear> {
    SchoolYear::new(2025).context("2025 is a season")
}

pub(super) fn observed_grade() -> Result<ObservedGrade> {
    Ok(ObservedGrade {
        grade: grade(11)?,
        school_year: season()?,
        source: SourceRef::id(SOURCE_ID),
    })
}

pub(super) fn team_of(school_id: &SchoolId, index: usize) -> Result<CanonicalTeam> {
    let id: TeamId = Id::mint("team", &[school_id.as_str(), "outdoor", &index.to_string()]);
    Ok(CanonicalTeam {
        id,
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Mixed,
        school_year: season()?,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![evidence()],
        retained_conflicts: Vec::new(),
    })
}

pub(super) fn meet_of(state: UsJurisdiction, index: usize) -> CanonicalMeet {
    let mut meet = CanonicalMeet::new(
        Some(state),
        format!("Synthetic Invite {index}"),
        MEET_DATE,
        CompetitionLevel::Invitational,
    );
    meet.evidence.push(evidence());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "synthetic_timer".to_string(),
        },
        format!("meet-{index}"),
    ));
    meet
}

pub(super) fn event_kind(value: u32) -> EventKind {
    match value % 3 {
        0 => EventKind::Track800m,
        1 => EventKind::Track1600m,
        _ => EventKind::Track3200m,
    }
}

pub(super) fn base_seconds(kind: &EventKind, rng: &mut Lcg) -> f64 {
    let base = match kind {
        EventKind::Track800m => 130.0,
        EventKind::Track1600m => 290.0,
        _ => 620.0,
    };
    base + f64::from(rng.next() % 500) / 100.0
}

pub(super) fn place_of(value: u32) -> Result<u16> {
    let lane = u16::try_from(value % 8).context("lane does not fit u16")?;
    Ok(lane.saturating_add(1))
}
