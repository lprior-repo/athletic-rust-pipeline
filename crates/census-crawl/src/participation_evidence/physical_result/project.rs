use super::super::result_ownership::{CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1};
use super::{
    one, provenance, HtmlResultProjectionError as Error, ProposedHtmlResultIdentityV1,
    ProposedHtmlResultTargetV1, Result, UnadmittedHtmlResultRowV1,
};
use census_domain::model::{
    AthleteId, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventIdentity,
    EventSpecification, Evidence, PerformanceIdentity, PerformanceResult, SourceEventLabel,
    SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;

pub(super) fn meet(document: &CheckedRetainedResultDocumentV1<'_>) -> Result<CanonicalMeet> {
    let page = document.parsed_page();
    let state = page
        .region
        .as_deref()
        .and_then(UsJurisdiction::parse)
        .ok_or(Error::Invalid("result jurisdiction unresolved"))?;
    let mut meet = CanonicalMeet::new_checked(
        Some(state),
        &page.meet.name,
        document.published_start(),
        crate::wiaa_results::level_of(&page.meet.name),
    )
    .map_err(Error::Meet)?;
    meet.end_date = page.meet.end_date.clone();
    meet.sports = one(page.sport.ok_or(Error::Invalid("result sport absent"))?)?;
    meet.source_urls = one(document.capture().requested_url().into())?;
    meet.source_identities = one(SourceIdentity::new(
        SourceNamespace::MilesplitMeet,
        document.meet_id(),
    ))?;
    meet.evidence = one(provenance::document_evidence(document)?)?;
    Ok(meet)
}

pub(super) fn row(
    document: &CheckedRetainedResultDocumentV1<'_>,
    meet: &CanonicalMeet,
    source: &CheckedRetainedResultRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<UnadmittedHtmlResultRowV1> {
    let key = provenance::source_key(document, source)?;
    let evidence = provenance::row_evidence(document, source, target, &key)?;
    let event = event(meet, source, &evidence)?;
    let team = team(document, source, target, &evidence)?;
    let parsed = source.parsed_row();
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: target.proposed_athlete,
            event: &event.id,
            meet: &meet.id,
            date: document.published_start(),
            source_key: &key,
        },
        PerformanceResult {
            team: &team.id,
            mark: parsed.mark.clone(),
            wind_mps: parsed.wind_mps,
            place: parsed.place,
            adjustment: None,
        },
    )?;
    if !performance.mark_compatible(&event.kind) {
        return Err(Error::Invalid("source mark conflicts with event kind"));
    }
    performance.heat = parsed.heat.clone();
    performance.round = source.event().round.clone();
    performance.timing = parsed.timing;
    performance.observed_grade = parsed.grade;
    performance.evidence = one(evidence)?;
    Ok(UnadmittedHtmlResultRowV1 {
        locator: source.locator(),
        subject_reference: target.subject_reference.into(),
        proposed_attachment_reference: target.proposed_attachment_reference.into(),
        proposed_school_binding_reference: target.proposed_school_binding_reference.into(),
        event,
        team,
        performance,
    })
}

fn event(
    meet: &CanonicalMeet,
    source: &CheckedRetainedResultRowV1<'_>,
    evidence: &Evidence,
) -> Result<CanonicalEvent> {
    let mut event = event_identity(meet, source)?;
    event.source_labels = one(SourceEventLabel {
        source: evidence.source.clone(),
        label: source.event().label.clone(),
    })?;
    event.evidence = one(evidence.clone())?;
    Ok(event)
}

fn event_identity(
    meet: &CanonicalMeet,
    source: &CheckedRetainedResultRowV1<'_>,
) -> Result<CanonicalEvent> {
    let parsed = source.event();
    let specification = EventSpecification::from_published_label(&parsed.label, &parsed.kind)?;
    Ok(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: parsed.kind.clone(),
            gender: parsed.gender,
            division: parsed.division.as_deref(),
            round: parsed.round.as_deref(),
        },
        specification,
    )?)
}

pub(super) fn identity(
    document: &CheckedRetainedResultDocumentV1<'_>,
    source: &CheckedRetainedResultRowV1<'_>,
    athlete: &AthleteId,
) -> Result<ProposedHtmlResultIdentityV1> {
    let meet = meet(document)?;
    let event = event_identity(&meet, source)?;
    let source_key = provenance::source_key(document, source)?;
    let performance = CanonicalPerformance::mint(
        athlete,
        &meet.id,
        &event.id,
        document.published_start(),
        &source_key,
    );
    Ok(ProposedHtmlResultIdentityV1 {
        performance,
        athlete: athlete.clone(),
        meet: meet.id,
        event: event.id,
        date: document.published_start().into(),
        source_key,
    })
}

fn team(
    document: &CheckedRetainedResultDocumentV1<'_>,
    source: &CheckedRetainedResultRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
    evidence: &Evidence,
) -> Result<CanonicalTeam> {
    let page = document.parsed_page();
    let sport = page.sport.ok_or(Error::Invalid("result sport absent"))?;
    let gender = source.event().gender;
    Ok(CanonicalTeam {
        id: CanonicalTeam::mint(target.proposed_school, sport, gender, page.school_year),
        school: target.proposed_school.clone(),
        sport,
        gender,
        school_year: page.school_year,
        level: source.event().division.clone(),
        source_identities: Vec::new(),
        evidence: one(evidence.clone())?,
        retained_conflicts: Vec::new(),
    })
}
