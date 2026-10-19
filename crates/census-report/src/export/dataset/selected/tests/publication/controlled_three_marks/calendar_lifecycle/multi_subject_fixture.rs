use super::super::documentary_material::{self, Kind};
use super::super::objects::{self, Objects};
use super::super::source::{self, InstalledSource, TestResult};
use census_crawl::participation_evidence::provider_target::SourceTargetPacketRefV1;
use census_crawl::participation_evidence::recorded::{
    read_recorded_independent_participation, read_recorded_provider_target,
    record_independent_participation_packet,
};
use census_crawl::participation_evidence::{
    IndependentParticipationPacketV1, ParticipationEvidenceRefV1,
};
use census_crawl::Recording;
use census_domain::evidence_subject_attachment::{
    AttachmentCorroborationBasisV1, AttachmentCorroborationPairV1, AttachmentDigestV1,
    AttachmentFactDispositionV1, AttachmentListV1, AttachmentPredecessorV1,
    AttachmentRecordReferenceV1, AttachmentReplayGrantV1, AttachmentReviewActorV1,
    AttachmentTextV1, RetainedDocumentSubjectReferenceV1,
    RetainedEvidenceSubjectAttachmentCommandV1, ReviewedAttachmentContextV1,
    ReviewedAttachmentFactV1, EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1,
};
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, JournalReadBudget, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};
pub(super) fn sibling_may15(
    installed: &InstalledSource,
    base: &ParticipationEvidenceRefV1,
) -> TestResult<ParticipationEvidenceRefV1> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let recorded = read_recorded_independent_participation(&snapshot, base, &mut grant)?
        .ok_or("sibling base absent")?;
    let day = recorded.checked().day().as_str().to_owned();
    let display = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d")?
        .format("%B %-d, %Y")
        .to_string();
    let root = recorded.root().clone();
    let raw = snapshot
        .journal_payload_raw_bounded(
            root.schema.phase(),
            &root.key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("sibling packet payload absent")?;
    let packet: IndependentParticipationPacketV1 = serde_json::from_str(raw.get())?;
    let schedule = packet.schedule().clone();
    let url = "https://www.mhsaa.com/sports/boys-track-field/stories/controlled-point-99";
    let path = "/sports/boys-track-field/stories/controlled-point-99";
    let body = format!("<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\"></head><body><article class=\"node__article\" data-path=\"{path}\" data-nid=\"100099\"><p class=\"publish-date\">{display}</p><h4 data-controlled-sibling=\"b\"><span class=\"text--secondary\">Controlled Target \u{2666} Detroit Catholic Central | Junior \u{2666} Track</span></h4><p>SYNTHETIC CONTROLLED FIXTURE. No genuine publication or authority. Point 50.01 recorded on {day}.</p><p>Corroborates provider profile at https://mi.milesplit.com/athletes/9999999-controlled-target for independent review.</p></article></body></html>");
    let status = source::capture(url, body.as_bytes(), b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: independent-participation\r\n\r\n", "2026-10-16T00:12:02Z", "2026-10-16T00:12:03Z")?;
    let sibling = IndependentParticipationPacketV1::new(status, schedule)?;
    let recording = Recording::new();
    record_independent_participation_packet(&installed.store, &recording, &sibling)?;
    census_crawl::recording::prepare_recorded(
        recording.drain(),
        "controlled-independent-intake-sibling",
    )?
    .apply(&installed.store)?;
    Ok(sibling.reference()?)
}
fn pair_reviewed(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    material_a: &documentary_material::SourceMaterial,
    material_b: &documentary_material::SourceMaterial,
    target: &(AttachmentDigestV1, AttachmentDigestV1, AttachmentDigestV1),
) -> TestResult<ReviewedAttachmentContextV1> {
    let rationale_a = "Controlled original owned documentary statement explicitly reviewed for this source-bound target";
    let rationale_t =
        "Actual retained provider person and public inspection witness; no accepted facts injected";
    let explanation = "Explicit controlled person-specific documentary review across retained MHSAA and provider sources";
    let mut facts = vec![
        ReviewedAttachmentFactV1 {
            fact: material_a.fact.clone(),
            subject: material_a.subject.subject.clone(),
            publisher_lineage: material_a.lineage.clone(),
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: AttachmentTextV1::parse(rationale_a)?,
        },
        ReviewedAttachmentFactV1 {
            fact: material_b.fact.clone(),
            subject: material_b.subject.subject.clone(),
            publisher_lineage: material_b.lineage.clone(),
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: AttachmentTextV1::parse(rationale_a)?,
        },
        ReviewedAttachmentFactV1 {
            fact: target.1.clone(),
            subject: target.0.clone(),
            publisher_lineage: target.2.clone(),
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: AttachmentTextV1::parse(rationale_t)?,
        },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let mut pairs = vec![
        AttachmentCorroborationPairV1 {
            subject_fact: material_a.fact.clone(),
            accepted_target_fact: target.1.clone(),
            basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext,
            explanation: AttachmentTextV1::parse(explanation)?,
        },
        AttachmentCorroborationPairV1 {
            subject_fact: material_b.fact.clone(),
            accepted_target_fact: target.1.clone(),
            basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext,
            explanation: AttachmentTextV1::parse(explanation)?,
        },
    ];
    pairs.sort_by(|left, right| left.subject_fact.cmp(&right.subject_fact));
    let mut subjects = vec![material_a.subject.clone(), material_b.subject.clone()];
    subjects.sort_by(|left, right| left.subject.cmp(&right.subject));
    let scope = objects.json(&(
        "controlled-documentary-complete-scope/v1",
        &subjects,
        &facts,
        &pairs,
        &target.0,
        inspection.target(),
        inspection.accepted_target_witness(),
    ))?;
    let mut references = objects.references()?;
    references.extend(inspection.issued_objects().cloned());
    references.sort();
    references.dedup();
    Ok(ReviewedAttachmentContextV1 {
        objects: AttachmentListV1::new(references)?,
        facts: AttachmentListV1::new(facts)?,
        corroboration: AttachmentListV1::new(pairs)?,
        complete_scope_records: AttachmentListV1::new(vec![scope])?,
        accepted_target_subjects: AttachmentListV1::new(vec![target.0.clone()])?,
        accepted_target_witness: inspection.accepted_target_witness().clone(),
    })
}
fn pair_commit(
    installed: &InstalledSource,
    inspection: EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &Objects,
    reviewed: ReviewedAttachmentContextV1,
    subjects: Vec<RetainedDocumentSubjectReferenceV1>,
) -> TestResult<super::super::documentary_review::AcceptedSource> {
    let command = RetainedEvidenceSubjectAttachmentCommandV1 { version: 1, policy: AttachmentTextV1::parse(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: inspection.store_identity().clone(), operation: AttachmentTextV1::parse("controlled-calendar-multi-subject")?, expected_public_head: inspection.public_head().clone(), expected_evidence_generation: inspection.evidence_generation(), expected_derived_generation: inspection.derived_generation(), expected_attachment_inventory: inspection.inventory().clone(), target: inspection.target().clone(), subjects: AttachmentListV1::new(subjects)?, action: inspection.action().clone(), reviewed_context: reviewed, actor: AttachmentReviewActorV1::LocalOperator, reviewed_at: chrono::DateTime::parse_from_rfc3339("2026-10-17T01:00:00Z")?.with_timezone(&chrono::Utc), rationale: AttachmentTextV1::parse("Synthetic controlled multi-subject review through actual public operations; same target with two distinct packet roots")? };
    let context = objects.borrowed_new(&installed.store)?;
    let prepared = installed.store.prepare_evidence_subject_attachment_v1(
        command.clone(),
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let receipt = match installed
        .store
        .apply_evidence_subject_attachment_v1(prepared, &approved, &digest)?
    {
        Application::Written(receipt) => receipt,
        Application::Repeated(_) => return Err("multi-subject unexpectedly replayed".into()),
    };
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let records = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    let record = records
        .records()
        .iter()
        .find(|record| record.command.operation == command.operation)
        .ok_or("multi-subject record absent")?;
    Ok(super::super::documentary_review::AcceptedSource {
        command,
        receipt,
        reference: AttachmentRecordReferenceV1 {
            operation: record.command.operation.clone(),
            digest: record.record_digest.clone(),
        },
    })
}
pub(super) fn accept_pair(
    installed: &InstalledSource,
    first: &ParticipationEvidenceRefV1,
    second: &ParticipationEvidenceRefV1,
) -> TestResult<super::super::documentary_review::AcceptedSource> {
    let inspection = installed.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: installed.public.clone(),
            prior: AttachmentPredecessorV1::First,
        },
    )?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let mut objects = Objects::default();
    let material_a =
        documentary_material::read(&snapshot, first, Kind::Point, &mut objects, &mut grant)?;
    let material_b =
        documentary_material::read(&snapshot, second, Kind::Point, &mut objects, &mut grant)?;
    let key = installed
        .source_records
        .iter()
        .find(|record| record.schema == RetainedCaptureRootSchemaV1::ProviderTargetPacketV1)
        .map(|record| record.key.as_str())
        .ok_or("sibling provider packet absent")?;
    let provider = read_recorded_provider_target(
        &snapshot,
        &SourceTargetPacketRefV1::parse(key)?,
        &mut grant,
    )?
    .ok_or("sibling provider checked absent")?;
    let raw = snapshot
        .journal_payload_raw_bounded(
            provider.root().schema.phase(),
            &provider.root().key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("sibling provider payload absent")?;
    #[derive(serde::Deserialize)]
    struct Profile<'a> {
        #[serde(borrow)]
        profile: &'a serde_json::value::RawValue,
    }
    let profile: Profile<'_> = serde_json::from_str(raw.get())?;
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: provider.root().clone(),
        slot: RetainedCaptureSlotV1::ProviderProfile,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: objects::reference(profile.profile.get().as_bytes())?,
    })?;
    let checked = provider.checked().profile();
    let target_subject = objects.json(&(
        "reviewed-original-person-specific-target/v1",
        inspection.accepted_target_witness(),
        checked.person(),
        checked.name(),
        provider.root(),
        &capture,
    ))?;
    let target_fact = objects.json(&(
        "controlled-original-source-bound-person/v1",
        checked.person(),
        checked.name(),
        provider.root(),
        &capture,
    ))?;
    let target_lineage = objects.json(&(
        "reviewed-publisher-family/v1",
        "milesplit.com",
        provider.root(),
    ))?;
    let mut subjects = vec![material_a.subject.clone(), material_b.subject.clone()];
    subjects.sort_by(|left, right| left.subject.cmp(&right.subject));
    let reviewed = pair_reviewed(
        &inspection,
        &mut objects,
        &material_a,
        &material_b,
        &(target_subject, target_fact, target_lineage),
    )?;
    pair_commit(installed, inspection, &objects, reviewed, subjects)
}
