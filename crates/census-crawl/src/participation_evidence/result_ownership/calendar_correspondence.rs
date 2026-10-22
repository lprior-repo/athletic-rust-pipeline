mod binding;
mod objects;
mod proof;
pub mod read;
mod result;
use binding::{retain_calendar_original_body, retain_result_context, status_binding};
use objects::{domain, guarded, invalid, object_reference, Objects};

use crate::participation_evidence::recorded::{
    read_recorded_independent_participation, with_current_attached_participation,
    CheckedDocumentaryParticipationFactsV1, DiscoveredIdentityLinkedHtmlResultDocumentV1,
    RecordedIndependentParticipationV1,
};
use crate::participation_evidence::result_ownership::CheckedHtmlResultDocumentV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentRecordReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::result_calendar_correspondence::{
    ResultCalendarEntryClaimV1, ResultCalendarSourceClaimV1,
};
use census_store::{
    AttachmentContextObjectV1, CurrentEvidenceSubjectAttachmentReferencesV1, RetainedCaptureRootV1,
};

pub struct ResultCalendarSourceProposalV1 {
    source: ResultCalendarSourceClaimV1,
    objects: Objects,
}

impl ResultCalendarSourceProposalV1 {
    pub fn claim(&self) -> &ResultCalendarSourceClaimV1 {
        &self.source
    }

    pub fn borrowed_context(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Vec<AttachmentContextObjectV1<'_>>> {
        guarded(grant, |grant| self.objects.borrowed(grant))
    }
}

pub fn build_claim(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    index: usize,
    requested: &RecordedIndependentParticipationV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarSourceProposalV1> {
    guarded(grant, |grant| {
        let row = linked
            .row(index)
            .ok_or_else(|| invalid("calendar result row absent"))?;
        let attached = row.attachment();
        super::original::current(current, attached, grant)?;
        let recorded =
            super::original::reopen(current.snapshot(), linked, row.checked_row(), grant)?;
        let document =
            recorded.read_source(&[row.checked_row().locator()], row.checked_row().source())?;
        with_current_attached_participation(current, grant, |attached_status, grant| {
            attached_status.with_row_facts(linked, index, grant, |facts, grant| {
                build_source(
                    current, linked, attached, &document, facts, requested, grant,
                )
            })
        })
    })
}

fn build_source(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
    attached: &crate::participation_evidence::recorded::DiscoveredResultAttachmentV1<'_, '_>,
    document: &CheckedHtmlResultDocumentV1<'_>,
    facts: CheckedDocumentaryParticipationFactsV1<'_>,
    requested: &RecordedIndependentParticipationV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarSourceProposalV1> {
    let checked = document
        .row(0)
        .ok_or_else(|| invalid("calendar reopened result row absent"))?;
    super::original::subject(linked.recorded_root(), &checked, attached, grant)?;
    let parser = result::parser(document, &checked)?;
    let scope = result::scope(document, &checked)?;
    let mut objects = Objects::new(grant)?;
    let result = result::claim(
        linked.recorded_root(),
        document,
        &checked,
        &mut objects,
        grant,
    )?;
    retain_result_context(current, attached, &mut objects, grant)?;
    let (documentary_attachment, documentary_root, documentary) = documentary_for(
        current,
        attached.target(),
        &facts,
        requested,
        &mut objects,
        grant,
    )?;
    validate_parser_documentary_match(&parser, documentary)?;
    let calendar = calendar_claim(documentary, &documentary_root, &mut objects, grant)?;
    retain_calendar_original_body(
        requested,
        &documentary_root,
        documentary,
        &mut objects,
        grant,
    )?;
    let parser_contract = objects.bytes(parser.contract(), grant)?;
    let source = ResultCalendarSourceClaimV1 {
        result,
        calendar,
        person: attached.target().clone(),
        result_attachment: attached.record().reference(),
        documentary_attachment,
        attachment_inventory: current.inventory().clone(),
        day: scope.day,
        sport: scope.sport,
        category: scope.category,
        year: scope.year,
        evidence: objects.references(grant)?,
        parser_contract,
    };
    Ok(ResultCalendarSourceProposalV1 { source, objects })
}

fn documentary_for<'a>(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    target: &census_domain::evidence_subject_attachment::AttachmentTargetV1,
    facts: &'a CheckedDocumentaryParticipationFactsV1<'_>,
    requested: &RecordedIndependentParticipationV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<(
    AttachmentRecordReferenceV1,
    RetainedCaptureRootV1,
    &'a crate::participation_evidence::VerifiedIndependentParticipationV1,
)> {
    let mut matched = None;
    for candidate in facts.sources() {
        grant.charge_steps(1).map_err(domain)?;
        if candidate.source().reference() != requested.checked().reference() {
            continue;
        }
        let reopened = read_recorded_independent_participation(
            current.snapshot(),
            candidate.source().reference(),
            grant,
        )?
        .ok_or_else(|| invalid("calendar documentary source absent"))?;
        let binding = status_binding(current, target, candidate, &reopened, objects, grant)?;
        if matched.is_some() {
            return Err(invalid("calendar requested status is ambiguous"));
        }
        matched = Some((binding, candidate.source()));
    }
    let ((attachment, root), source) =
        matched.ok_or_else(|| invalid("calendar requested status absent"))?;
    if requested.root() != &root {
        return Err(invalid("calendar requested status root differs"));
    }
    Ok((attachment, root, source))
}

fn calendar_claim(
    source: &crate::participation_evidence::VerifiedIndependentParticipationV1,
    root: &RetainedCaptureRootV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarEntryClaimV1> {
    let proof = source.season_proof();
    let interpretation = proof::preimage(source, true)?;
    let expected = object_reference(interpretation.as_bytes(), grant)?;
    if expected.digest.as_str() != proof.interpretation() {
        return Err(invalid("calendar season interpretation digest differs"));
    }
    let held = objects.bytes(interpretation.as_bytes(), grant)?;
    if held != expected {
        return Err(invalid("calendar season interpretation differs"));
    }
    let locator = objects.bytes(proof.locator().as_bytes(), grant)?;
    Ok(ResultCalendarEntryClaimV1 {
        documentary_root: objects.json(root, grant)?,
        capture: AttachmentDigestV1::parse(proof.capture()).map_err(domain)?,
        interpretation: AttachmentDigestV1::parse(proof.interpretation()).map_err(domain)?,
        locator: locator.digest,
        scope: census_domain::evidence_subject_attachment::AttachmentTextV1::parse(
            source.season_scope(),
        )
        .map_err(domain)?,
    })
}

fn validate_parser_documentary_match(
    parser: &census_domain::result_calendar_correspondence::ResultCalendarSourceParserV1,
    documentary: &crate::participation_evidence::VerifiedIndependentParticipationV1,
) -> CrawlResult<()> {
    if parser
        .documentary_interpretation(
            documentary.subject().document_namespace(),
            documentary.subject().document_namespace() == "milesplit_result_document_v1",
            documentary.roncalli_event().is_some(),
        )
        .is_none()
    {
        return Err(invalid("calendar parser and documentary source differ"));
    }
    Ok(())
}
