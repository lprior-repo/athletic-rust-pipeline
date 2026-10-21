mod binding;
mod objects;
pub mod read;
use binding::{retain_calendar_original_body, retain_result_context, status_binding};
use objects::{digest, domain, guarded, invalid, object_reference, Objects};

fn proof_interpretation_name(
    subject: &crate::participation_evidence::DocumentSubjectV1,
) -> CrawlResult<&'static str> {
    match subject.document_namespace() {
        "mhsaa_article_v1" => Ok("mhsaa_subject_dcc_event_day_v1"),
        "milesplit_result_document_v1" => Ok("milesplit_original_student_grade_dcc_event_day_v1"),
        _ => Err(invalid("unsupported documentary status namespace")),
    }
}
use crate::participation_evidence::recorded::{
    read_recorded_independent_participation, with_current_attached_participation,
    CheckedDocumentaryParticipationFactsV1, DiscoveredIdentityLinkedHtmlResultDocumentV1,
    RecordedIndependentParticipationV1,
};
use crate::participation_evidence::result_ownership::{
    CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentRecordReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};
use census_domain::result_calendar_correspondence::{
    ResultCalendarEntryClaimV1, ResultCalendarRowClaimV1, ResultCalendarSourceClaimV1,
    RESULT_CALENDAR_PARSER_CONTRACT_V1,
};
use census_store::{
    AttachmentContextObjectV1, CurrentEvidenceSubjectAttachmentReferencesV1,
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
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
        let document = recorded.read(&[row.checked_row().locator()])?;
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
    document: &CheckedRetainedResultDocumentV1<'_>,
    facts: CheckedDocumentaryParticipationFactsV1<'_>,
    requested: &RecordedIndependentParticipationV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarSourceProposalV1> {
    let checked = document
        .row(0)
        .ok_or_else(|| invalid("calendar reopened result row absent"))?;
    super::original::subject(linked.recorded_root(), &checked, attached, grant)?;
    let scope = scope(document, &checked)?;
    let mut objects = Objects::new(grant)?;
    let result = result_claim(
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
    let calendar = calendar_claim(documentary, &documentary_root, &mut objects, grant)?;
    retain_calendar_original_body(
        requested,
        &documentary_root,
        documentary,
        &mut objects,
        grant,
    )?;
    let parser_contract = objects.bytes(RESULT_CALENDAR_PARSER_CONTRACT_V1, grant)?;
    let source = ResultCalendarSourceClaimV1 {
        result,
        calendar,
        person: attached.target().clone(),
        result_attachment: AttachmentRecordReferenceV1 {
            operation: attached.record().command.operation.clone(),
            digest: attached.record().record_digest.clone(),
        },
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
        let reopened = read_recorded_independent_participation(
            current.snapshot(),
            candidate.source().reference(),
            grant,
        )?
        .ok_or_else(|| invalid("calendar documentary source absent"))?;
        let binding = status_binding(current, target, candidate, &reopened, objects, grant)?;
        if candidate.source().reference() == requested.checked().reference() {
            if matched.is_some() {
                return Err(invalid("calendar requested status is ambiguous"));
            }
            matched = Some((binding, candidate.source()));
        }
    }
    let ((attachment, root), source) =
        matched.ok_or_else(|| invalid("calendar requested status absent"))?;
    if requested.root() != &root {
        return Err(invalid("calendar requested status root differs"));
    }
    Ok((attachment, root, source))
}

struct Scope {
    day: ProfileDate,
    sport: Sport,
    category: Gender,
    year: SchoolYear,
}

fn scope(
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
) -> CrawlResult<Scope> {
    let start = crate::context::published_performance_date(document.published_start())
        .ok_or_else(|| invalid("calendar exact performance day unresolved"))?;
    if document
        .published_end()
        .map(crate::context::published_performance_date)
        .is_some_and(|end| end != Some(start))
    {
        return Err(invalid("calendar result spans multiple days"));
    }
    let day_text = start.format("%Y-%m-%d").to_string();
    let day = ProfileDate::parse(&day_text).map_err(|_| invalid("calendar result day invalid"))?;
    let sport = document
        .parsed_page()
        .sport
        .ok_or_else(|| invalid("calendar result sport absent"))?;
    let category = row.event().gender;
    if category == Gender::Unknown {
        return Err(invalid("calendar result category unknown"));
    }
    let year = document.parsed_page().school_year;
    if SchoolYear::from_date(day.as_str()) != Some(year) {
        return Err(invalid("calendar result day and year disagree"));
    }
    Ok(Scope {
        day,
        sport,
        category,
        year,
    })
}

fn result_claim(
    root: &RetainedCaptureRootV1,
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarRowClaimV1> {
    let body = object_reference(document.original_document().as_bytes(), grant)?;
    if body.digest.as_str() != document.capture().body_sha256() {
        return Err(invalid("calendar original result body differs"));
    }
    objects.retain(
        RetainedCaptureLeafV1 {
            version: 1,
            root: root.clone(),
            slot: RetainedCaptureSlotV1::SelfCapture,
            kind: RetainedCaptureLeafKindV1::OriginalBody,
            object: body.clone(),
        },
        grant,
    )?;
    let interpretation = objects.json(
        &(
            "retained-milesplit-html-result-row/v1",
            document.capture().requested_url(),
            document.capture().body_sha256(),
            row.locator(),
        ),
        grant,
    )?;
    if interpretation.digest.as_str() != row.reference() {
        return Err(invalid("calendar result interpretation differs"));
    }
    let subject = row
        .subject()
        .map_err(|_| invalid("calendar result subject unavailable"))?;
    Ok(ResultCalendarRowClaimV1 {
        result_root: objects.json(root, grant)?,
        body,
        interpretation,
        row_locator: digest(subject.locator().as_bytes(), grant)?,
        row_digest: AttachmentDigestV1::parse(row.original_row_digest()).map_err(domain)?,
    })
}

fn calendar_claim(
    source: &crate::participation_evidence::VerifiedIndependentParticipationV1,
    root: &RetainedCaptureRootV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ResultCalendarEntryClaimV1> {
    let proof = source.season_proof();
    let name = proof_interpretation_name(source.subject())?;
    let interpretation = format!("{name}\n{}\n{}", proof.capture(), proof.locator());
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
