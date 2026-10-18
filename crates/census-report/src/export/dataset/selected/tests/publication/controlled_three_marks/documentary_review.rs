use super::documentary_material::{self, Kind, SourceMaterial};
use super::objects::{self, Objects};
use super::source::{InstalledSource, TestResult};
use census_crawl::participation_evidence::provider_target::SourceTargetPacketRefV1;
use census_crawl::participation_evidence::recorded::read_recorded_provider_target;
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
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
    EvidenceSubjectAttachmentPreparationV1, JournalReadBudget, Receipt, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1, Store,
    StoreSnapshot,
};
use chrono::{DateTime, Utc};

pub(super) struct AcceptedSource {
    pub(super) command: RetainedEvidenceSubjectAttachmentCommandV1,
    pub(super) receipt: Receipt,
    pub(super) reference: AttachmentRecordReferenceV1,
}

struct Target {
    subject: AttachmentDigestV1,
    fact: AttachmentDigestV1,
    lineage: AttachmentDigestV1,
}

pub(super) fn accept(
    installed: &InstalledSource,
    reference: &ParticipationEvidenceRefV1,
    kind: Kind,
    prior: AttachmentPredecessorV1,
    operation: &str,
) -> TestResult<AcceptedSource> {
    let inspection = installed.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: installed.public.clone(),
            prior,
        },
    )?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let mut objects = Objects::default();
    let material =
        documentary_material::read(&snapshot, reference, kind, &mut objects, &mut grant)?;
    let target = target(installed, &snapshot, &inspection, &mut objects, &mut grant)?;
    let reviewed = reviewed(&inspection, &mut objects, &material, target)?;
    let command = command(&inspection, material.subject, reviewed, operation)?;
    commit(&installed.store, inspection, command, &objects)
}

fn target(
    installed: &InstalledSource,
    snapshot: &StoreSnapshot<'_>,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Target> {
    let key = installed
        .source_records
        .iter()
        .find(|record| record.schema == RetainedCaptureRootSchemaV1::ProviderTargetPacketV1)
        .map(|record| record.key.as_str())
        .ok_or("original source-bound provider packet missing")?;
    let provider =
        read_recorded_provider_target(snapshot, &SourceTargetPacketRefV1::parse(key)?, grant)?
            .ok_or("checked provider packet missing")?;
    let raw = snapshot
        .journal_payload_raw_bounded(
            provider.root().schema.phase(),
            &provider.root().key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("provider original payload missing")?;
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
    let subject = objects.json(&(
        "reviewed-original-person-specific-target/v1",
        inspection.accepted_target_witness(),
        checked.person(),
        checked.name(),
        provider.root(),
        &capture,
    ))?;
    let fact = objects.json(&(
        "controlled-original-source-bound-person/v1",
        checked.person(),
        checked.name(),
        provider.root(),
        &capture,
    ))?;
    let lineage = objects.json(&(
        "reviewed-publisher-family/v1",
        "milesplit.com",
        provider.root(),
    ))?;
    Ok(Target {
        subject,
        fact,
        lineage,
    })
}

fn reviewed(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    material: &SourceMaterial,
    target: Target,
) -> TestResult<ReviewedAttachmentContextV1> {
    let mut facts = vec![
        ReviewedAttachmentFactV1 { fact: material.fact.clone(), subject: material.subject.subject.clone(), publisher_lineage: material.lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Controlled original owned documentary statement explicitly reviewed for this source-bound target")? },
        ReviewedAttachmentFactV1 { fact: target.fact.clone(), subject: target.subject.clone(), publisher_lineage: target.lineage, disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Actual retained provider person and public inspection witness; no accepted facts injected")? },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let pairs = vec![AttachmentCorroborationPairV1 { subject_fact: material.fact.clone(), accepted_target_fact: target.fact, basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext, explanation: text("Explicit controlled person-specific documentary review across retained MHSAA and provider sources")? }];
    let scope = objects.json(&(
        "controlled-documentary-complete-scope/v1",
        &material.subject,
        &facts,
        &pairs,
        &target.subject,
        inspection.target(),
        inspection.accepted_target_witness(),
    ))?;
    let mut references = objects.references()?;
    references.extend(inspection.issued_objects().cloned());
    references.sort();
    references.dedup();
    Ok(ReviewedAttachmentContextV1 {
        objects: list(references)?,
        facts: list(facts)?,
        corroboration: list(pairs)?,
        complete_scope_records: list(vec![scope])?,
        accepted_target_subjects: list(vec![target.subject])?,
        accepted_target_witness: inspection.accepted_target_witness().clone(),
    })
}

fn command(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    subject: RetainedDocumentSubjectReferenceV1,
    reviewed_context: ReviewedAttachmentContextV1,
    operation: &str,
) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    Ok(RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: inspection.store_identity().clone(), operation: text(operation)?, expected_public_head: inspection.public_head().clone(), expected_evidence_generation: inspection.evidence_generation(), expected_derived_generation: inspection.derived_generation(), expected_attachment_inventory: inspection.inventory().clone(), target: inspection.target().clone(), subjects: list(vec![subject])?, action: inspection.action().clone(), reviewed_context,
        actor: AttachmentReviewActorV1::LocalOperator, reviewed_at: DateTime::parse_from_rfc3339("2026-10-17T01:00:00Z")?.with_timezone(&Utc), rationale: text("Synthetic controlled review through actual public operations, not authenticated human or genuine population acceptance")?,
    })
}

fn commit(
    store: &Store,
    inspection: EvidenceSubjectAttachmentInspectionV1<'_>,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    objects: &Objects,
) -> TestResult<AcceptedSource> {
    let context = objects.borrowed_new(store)?;
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command.clone(),
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let receipt = match store.apply_evidence_subject_attachment_v1(prepared, &approved, &digest)? {
        Application::Written(receipt) => receipt,
        Application::Repeated(_) => {
            return Err("controlled source unexpectedly replayed during first acceptance".into())
        }
    };
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let records = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    let record = records
        .records()
        .iter()
        .find(|record| record.command.operation == command.operation)
        .ok_or("written documentary source record missing")?;
    let reference = AttachmentRecordReferenceV1 {
        operation: record.command.operation.clone(),
        digest: record.record_digest.clone(),
    };
    Ok(AcceptedSource {
        command,
        receipt,
        reference,
    })
}

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}
fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}
