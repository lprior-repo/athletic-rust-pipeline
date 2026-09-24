//! The row shapes the synthetic corpus is built from: the label constants every row shares and one
//! constructor per shape — evidence, the observed grade, the team, the meet, the event kind, the
//! athlete's time and the lane it ran in.
//!
//! Each constructor is a pure function of its arguments (and of the seeded `Lcg`, for the jittered
//! time), so the corpus is reproducible from the seed alone.

use anyhow::{Context, Result};
use census_domain::model::{
    CanonicalMeet, CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, Grade, Id,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport, TeamId,
};
use census_domain::UsJurisdiction;

use super::lcg::Lcg;

/// Evidence source id for every synthetic row; a core adapter id so `Scope::Core` keeps the rows.
pub(super) const SOURCE_ID: &str = "mshsl_results";
pub(super) const MEET_DATE: &str = "2026-05-02";

pub(super) fn evidence() -> Evidence {
    Evidence::parsed(SourceRef::id(SOURCE_ID), MEET_DATE)
}

pub(super) fn grade(value: u8) -> Result<Grade> {
    Grade::new(value).context("grade must be between 9 and 12")
}

/// The season every synthetic row carries. `2025` sits inside the domain's year window by
/// construction, so the only way this fails is the constant disagreeing with the constructor.
pub(super) fn season() -> Result<SchoolYear> {
    SchoolYear::new(2025).context("2025 is a season")
}

/// The grade-11 observation every athlete row carries for the 2025 season, from [`SOURCE_ID`].
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

/// Three track events, so every meet holds a handful of distinct events rather than one.
pub(super) fn event_kind(value: u32) -> EventKind {
    match value % 3 {
        0 => EventKind::Track800m,
        1 => EventKind::Track1600m,
        _ => EventKind::Track3200m,
    }
}

/// A plausible time for the event, jittered from 0.00 to 4.99 seconds.
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
