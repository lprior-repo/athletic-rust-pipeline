use super::TestResult;
use census_crawl::participation_evidence::physical_result::project_retained_html_result_identity;
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, DiscoveredIdentityLinkedHtmlResultDocumentV1,
    with_current_attached_participation, AttachedParticipationV1,
    CheckedDocumentaryParticipationFactsV1, DocumentarySeasonApplicabilityV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    AthleteId, EventId, ExactSeconds, Gender, Mark, PerformanceId, TimingMethod,
};
use census_store::{Store, Table};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct Expected {
    pub performance: PerformanceId,
    pub event: EventId,
    pub athlete: AthleteId,
    pub references: Vec<String>,
    pub semantic_references: Vec<String>,
}

pub(super) fn read(store: &Store) -> TestResult<Vec<Expected>> {
    let snapshot = store.snapshot();
    check!(snapshot
        .scan::<census_domain::model::CanonicalPerformance>(Table::Performances)?
        .is_empty());
    check!(snapshot
        .scan::<census_domain::model::CanonicalTeam>(Table::Teams)?
        .is_empty());
    let mut grant = AttachmentReplayGrantV1::new();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("genuine accepted target")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    let mut expected = with_current_attached_participation(&current, &mut grant, |sources, grant| {
        plan.with_group(0, grant, |linked, grant| Ok(rows(&linked, sources, grant)))
    })??;
    expected.sort_by(|left, right| left.performance.cmp(&right.performance));
    check!(eq; expected.len(), 3);
    check!(expected
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left.performance < right.performance)));
    Ok(expected)
}

fn rows<'s>(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, 's>,
    sources: &AttachedParticipationV1<'_, 's>,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Vec<Expected>> {
    check!(eq; linked.row_count(), 3);
    let document = linked.checked_document();
    check!(eq; document.meet_id(), "763888");
    check!(eq; document.result_document_id(), "1304896");
    check!(eq; document.published_start(), "2026-05-15");
    let original = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html"));
    check!(eq; document.original_document().as_bytes(), original.as_slice());
    check!(!document.original_document().contains("7223777"));
    check!(!document.original_document().contains("12439"));
    let mut rows = Vec::new();
    for (index, (mark, wind, digest)) in [
        (
            "13.57",
            Some(0.2),
            "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d",
        ),
        (
            "14.10",
            None,
            "6bf39ca146977ced76ba63e226c1d41786e1914947f492c148fe3ca8708cd582",
        ),
        (
            "36.23",
            None,
            "8595ae83c23900e31b747b8bc0b3c6adb7c842139e0344f15e361a71e718d736",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let linked_row = linked.row(index).ok_or("genuine linked row")?;
        let row = linked_row.checked_row();
        let attachment = linked_row.attachment();
        attachment.require_original_grant(grant)?;
        check!(eq; row.parsed_row().mark, Mark::TimeSeconds(ExactSeconds::parse(mark)?));
        check!(eq; row.parsed_row().wind_mps, wind);
        check!(eq; row.parsed_row().timing, Some(TimingMethod::Fat));
        check!(eq; row.parsed_row().grade.map(|grade| grade.get()), Some(12));
        check!(eq; row.event().gender, Gender::Boys);
        check!(eq; row.original_row_digest(), digest);
        check!(eq; attachment.record().command.operation.as_str(), "genuine-reviewed-may15-b-v1");
        check!(eq; attachment.inventory().records, 2);
        let athlete: AthleteId = attachment.target().canonical_candidate.cast();
        let identity = project_retained_html_result_identity(document, index, &athlete)?;
        let subject = attachment.subject();
        let references = [
            subject.capture.as_str(),
            subject.body.as_str(),
            subject.interpretation.as_str(),
            subject.locator.as_str(),
            subject.context.as_str(),
            digest,
            attachment.record().record_digest.as_str(),
            attachment.inventory().digest.as_str(),
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let semantic_references = sources.with_row_facts(linked, index, grant, |facts, _| {
            Ok(semantic_references(facts))
        })??;
        rows.push(Expected {
            performance: identity.performance_id().clone(),
            event: identity.event_id().clone(),
            athlete,
            references,
            semantic_references,
        });
    }
    Ok(rows)
}

fn semantic_references(facts: CheckedDocumentaryParticipationFactsV1<'_>) -> TestResult<Vec<String>> {
    let [item] = facts.sources() else { return Err("one genuine active A status source".into()); };
    check!(eq; item.season(), DocumentarySeasonApplicabilityV1::CompetitionAndVenueMismatch);
    let source = item.source();
    check!(eq; source.cohort().get(), 2026);
    check!(eq; source.status_proof().capture(), "2fb479da65fb5cd69a3bb4a6b8a5625c298cfe51cdf06a43efe5359bf31de57a");
    check!(eq; source.status_proof().locator(), "html_bytes:113581:113939");
    check!(eq; source.season_proof().capture(), "26833e4c45506da5d776eb844b106dcfb041f67f7916bec0fb59c950b99693eb");
    let status = source.status_proof();
    let season = source.season_proof();
    Ok([source.reference().as_str(), item.attachment_record_digest(),
        status.capture(), status.interpretation(), status.locator(),
        season.capture(), season.interpretation(), season.locator(), source.season_scope()]
        .into_iter().map(str::to_owned).collect())
}
