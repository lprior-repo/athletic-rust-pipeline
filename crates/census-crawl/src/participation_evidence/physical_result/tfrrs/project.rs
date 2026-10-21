use super::{admission, provenance, CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1, Error, Result};
use crate::participation_evidence::physical_result::{
    one, ProposedHtmlResultIdentityV1, ProposedHtmlResultTargetV1, UnadmittedHtmlResultRowV1,
};
use crate::tfrrs::{mark_with_adjustment, ParsedMark};
use census_domain::model::{
    AthleteId, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CompetitionLevel, EventIdentity, EventSpecification, Evidence, Mark,
    PerformanceIdentity, PerformanceResult, SchoolYear, SourceEventLabel, SourceIdentity,
    SourceNamespace,
};

pub(super) fn meet(list: &CheckedRetainedTfrrsListV1<'_>, row: &CheckedRetainedTfrrsRowV1<'_>) -> Result<CanonicalMeet> {
    let mut meet = CanonicalMeet::new_checked(
        None, row.meet_name(), row.published_date(), CompetitionLevel::Unknown,
    ).map_err(Error::Meet)?;
    meet.sports = one(list.sport())?;
    meet.source_urls = one(list.capture().requested_url().into())?;
    meet.source_identities = one(SourceIdentity::new(SourceNamespace::TfrrsMeet, row.meet_id())
        .with_url(source_url(list, row.meet_path())?))?;
    meet.evidence = one(provenance::meet_evidence(list, row)?)?;
    Ok(meet)
}

pub(super) fn row(
    list: &CheckedRetainedTfrrsListV1<'_>,
    meet: &CanonicalMeet,
    source: &CheckedRetainedTfrrsRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<UnadmittedHtmlResultRowV1> {
    let key = provenance::source_key(list, source)?;
    let evidence = provenance::row_evidence(list, source, target, &key)?;
    let mut event = event(meet, source)?;
    event.source_labels = one(SourceEventLabel {
        source: evidence.source.clone(), label: source.event_label().into(),
    })?;
    event.evidence = one(evidence.clone())?;
    let team = team(list, source, target, &evidence)?;
    let mut performance = performance(meet, &event, &team, source, target, &key)?;
    performance.source_athlete = Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, source.athlete_id())
        .with_url(source_url(list, source.athlete_path())?));
    performance.observed_grade = Some(source.grade());
    performance.evidence = one(evidence)?;
    Ok(UnadmittedHtmlResultRowV1 {
        locator: source.locator(), subject_reference: target.subject_reference.into(),
        proposed_attachment_reference: target.proposed_attachment_reference.into(),
        proposed_school_binding_reference: target.proposed_school_binding_reference.into(),
        event, team, performance,
    })
}

fn performance(
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
    team: &CanonicalTeam,
    source: &CheckedRetainedTfrrsRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
    key: &str,
) -> Result<CanonicalPerformance> {
    let parsed = if event.kind.is_field() {
        ParsedMark::Field(source.published_mark().into())
    } else {
        ParsedMark::Time(source.published_mark().into())
    };
    let (mark, adjustment) = mark_with_adjustment(&parsed, None, source.conversion_note())
        .ok_or(Error::Invalid("unsupported TFRRS source mark"))?;
    if matches!(mark, Mark::Raw(_)) || source.conversion_note().is_some()
        && adjustment.as_ref().is_none_or(|value| value.original_mark.is_none()) {
        return Err(Error::Invalid("unsupported TFRRS conversion or mark"));
    }
    let performance = CanonicalPerformance::new(PerformanceIdentity {
        athlete: target.proposed_athlete, event: &event.id, meet: &meet.id,
        date: source.published_date(), source_key: key,
    }, PerformanceResult {
        team: &team.id, mark, wind_mps: None, place: None, adjustment,
    })?;
    if !performance.mark_compatible(&event.kind) {
        return Err(Error::Invalid("TFRRS source mark conflicts with event kind"));
    }
    Ok(performance)
}

fn event(meet: &CanonicalMeet, row: &CheckedRetainedTfrrsRowV1<'_>) -> Result<CanonicalEvent> {
    let kind = admission::kind(row)?;
    let specification = EventSpecification::from_published_label(row.event_label(), &kind)?;
    Ok(CanonicalEvent::new(EventIdentity {
        meet: &meet.id, kind, gender: row.gender(), division: None, round: None,
    }, specification)?)
}

fn team(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
    evidence: &Evidence,
) -> Result<CanonicalTeam> {
    let school_year = SchoolYear::from_date(row.published_date())
        .ok_or(Error::Invalid("TFRRS row date school year"))?;
    let native = crate::tfrrs::parse_team_path(row.team_path())
        .ok_or(Error::Invalid("checked TFRRS team path"))?;
    Ok(CanonicalTeam {
        id: CanonicalTeam::mint(target.proposed_school, list.sport(), row.gender(), school_year),
        school: target.proposed_school.clone(), sport: list.sport(), gender: row.gender(),
        school_year, level: None,
        source_identities: one(SourceIdentity::new(SourceNamespace::TfrrsTeam, native.slug)
            .with_url(source_url(list, row.team_path())?))?,
        evidence: one(evidence.clone())?, retained_conflicts: Vec::new(),
    })
}

pub(super) fn identity(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    athlete: &AthleteId,
) -> Result<ProposedHtmlResultIdentityV1> {
    let meet = meet(list, row)?;
    let event = event(&meet, row)?;
    let source_key = provenance::source_key(list, row)?;
    let performance = CanonicalPerformance::mint(athlete, &meet.id, &event.id,
        row.published_date(), &source_key);
    Ok(ProposedHtmlResultIdentityV1 {
        performance, athlete: athlete.clone(), meet: meet.id, event: event.id,
        date: row.published_date().into(), source_key,
    })
}

fn source_url(list: &CheckedRetainedTfrrsListV1<'_>, path: &str) -> Result<String> {
    let base = url::Url::parse(list.capture().requested_url())
        .map_err(|_| Error::Invalid("checked TFRRS source URL"))?;
    Ok(base.join(path).map_err(|_| Error::Invalid("checked TFRRS native path"))?.into())
}
