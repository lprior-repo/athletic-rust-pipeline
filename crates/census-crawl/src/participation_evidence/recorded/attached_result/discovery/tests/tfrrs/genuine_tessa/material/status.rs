use super::*;
use census_domain::result_calendar_correspondence::RoncalliSeasonEventProofContextV1;
use serde_json::value::RawValue;

pub(in super::super) fn read(
    installed: &source::Installed,
    objects: &mut objects::Objects,
) -> TestResult<Material> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let recorded =
        read_recorded_independent_participation(&snapshot, &installed.participation, &mut grant)?
            .ok_or("retained exact Roncalli pair")?;
    let checked = recorded.checked();
    let raw = snapshot
        .journal_payload_raw_bounded(
            recorded.root().schema.phase(),
            &recorded.root().key,
            JournalReadBudget::new(4 * 1024 * 1024)?,
        )?
        .ok_or("literal Roncalli packet")?;
    #[derive(serde::Deserialize)]
    struct Slots<'a> {
        #[serde(borrow)]
        status: &'a RawValue,
    }
    let slots: Slots<'_> = serde_json::from_str(raw.get())?;
    let document = checked.subject();
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: recorded.root().clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: objects::reference(slots.status.get().as_bytes())?,
    });
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: recorded.root().clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: objects::reference(roncalli_fixture::REVIEW.as_bytes())?,
    });
    let candidate = checked
        .roncalli_event()
        .ok_or("checked real Roncalli candidate")?;
    let selection = census_domain::result_calendar_correspondence::RoncalliSeasonEventSelectionV1 {
        status_subject_start: candidate.status().subject_span.start,
        status_subject_end: candidate.status().subject_span.end,
        calendar_recap_id: candidate.event().recap_id.clone(),
    };
    let proof = checked.status_proof();
    let interpretation = objects.bytes(
        selection
            .interpretation_preimage(RoncalliSeasonEventProofContextV1 {
                status_body_sha: proof.capture(),
                schedule_body_sha: checked.season_proof().capture(),
                status_locator: proof.locator(),
                calendar_locator: checked.season_proof().locator(),
                day: checked.day(),
            })
            .into_bytes(),
    )?;
    check!(eq; interpretation.as_str(), proof.interpretation());
    let subject_digest = subject(objects, document)?;
    let locator = objects.bytes(proof.locator().as_bytes().to_vec())?;
    let context = objects.json(recorded.root())?;
    let fact = objects.bytes(
        roncalli_fixture::REVIEW
            .get(candidate.status().statement_span.clone())
            .ok_or("literal 2026 junior statement")?
            .as_bytes()
            .to_vec(),
    )?;
    let source = read_recorded_tfrrs_list_capture(
        &snapshot,
        &installed.context,
        &[source::locator()],
        &mut grant,
    )?
    .ok_or("retained native target source")?;
    let original = source.read(&[source::locator()])?;
    let target_fact = objects.bytes(
        original
            .row(0)
            .ok_or("native Tessa row")?
            .original_row()?
            .as_bytes()
            .to_vec(),
    )?;
    let target_lineage =
        objects.json(&("genuine-tessa-publisher/v1", "tfrrs.org", source.root()))?;
    let lineage = objects.json(&(
        "genuine-tessa-publisher/v1",
        "athletics.roncalli.org",
        recorded.root(),
    ))?;
    Ok(Material {
        subject: RetainedDocumentSubjectReferenceV1 {
            namespace: text(document.document_namespace())?,
            native_id: Some(text(document.native_document_id())?),
            capture,
            body,
            interpretation,
            locator,
            subject: subject_digest,
            context,
            role: AttachmentSubjectRoleV1::AthleteStatus,
        },
        fact,
        lineage,
        target_fact,
        target_lineage,
        basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext,
    })
}
