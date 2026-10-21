use base64::Engine as _;
use census_crawl::participation_evidence::provider_target::SourceTargetPacketRefV1;
use census_crawl::participation_evidence::recorded::{
    read_recorded_independent_participation, read_recorded_provider_target,
    RecordedProviderTargetV1,
};
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentCorroborationBasisV1, AttachmentCorroborationPairV1,
    AttachmentDigestV1, AttachmentFactDispositionV1, AttachmentListV1, AttachmentObjectReferenceV1,
    AttachmentPredecessorV1, AttachmentReplayGrantV1, AttachmentReviewActorV1,
    AttachmentSubjectRoleV1, AttachmentTextV1, RetainedDocumentSubjectReferenceV1,
    RetainedEvidenceSubjectAttachmentCommandV1, ReviewedAttachmentContextV1,
    ReviewedAttachmentFactV1, EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1,
};
use census_domain::model::{serialized_bytes, AthleteId, GradYear, ProfileDate, SourceNamespace};
use census_domain::public_identity_history::{BindingResolution, HistoryRequest, PublicResolution};
use census_store::{
    Application, AttachmentContextObjectV1, AttachmentInspectionRequestV1,
    EvidenceSubjectAttachmentInspectionV1, EvidenceSubjectAttachmentPreparationV1,
    JournalReadBudget, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1, Store,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

fn text(value: &str) -> Result<AttachmentTextV1, Box<dyn std::error::Error>> {
    Ok(AttachmentTextV1::parse(value)?)
}

fn list<T>(values: Vec<T>) -> Result<AttachmentListV1<T>, Box<dyn std::error::Error>> {
    Ok(AttachmentListV1::new(values)?)
}

fn object_reference(
    bytes: &[u8],
) -> Result<AttachmentObjectReferenceV1, Box<dyn std::error::Error>> {
    let digest = AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?;
    let byte_count = u64::try_from(bytes.len())?;
    Ok(AttachmentObjectReferenceV1 {
        digest,
        bytes: byte_count,
    })
}

fn hold_inline(
    held: &mut Vec<HeldObject>,
    bytes: Vec<u8>,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<AttachmentDigestV1, Box<dyn std::error::Error>> {
    let reference = object_reference(&bytes)?;
    if let Some(existing) = held
        .iter()
        .find(|object| object.reference().digest == reference.digest)
    {
        if existing.reference() != &reference {
            return Err("conflicting inline object lengths".into());
        }
        return Ok(reference.digest);
    }
    grant.charge_steps(1)?;
    grant.charge_bytes(bytes.len())?;
    held.try_reserve(1)?;
    held.push(HeldObject::Inline {
        reference: reference.clone(),
        bytes,
    });
    Ok(reference.digest)
}

fn hold_retained(
    held: &mut Vec<HeldObject>,
    leaf: RetainedCaptureLeafV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<AttachmentDigestV1, Box<dyn std::error::Error>> {
    let digest = leaf.object.digest.clone();
    if let Some(existing) = held
        .iter()
        .find(|object| object.reference().digest == digest)
    {
        if !matches!(existing, HeldObject::Retained(existing_leaf) if existing_leaf == &leaf) {
            return Err("retained object digest has conflicting descriptors".into());
        }
        return Ok(digest);
    }
    grant.charge_steps(1)?;
    grant.charge_bytes(
        leaf.root
            .key
            .len()
            .checked_add(192)
            .ok_or("point leaf size overflow")?,
    )?;
    held.try_reserve(1)?;
    held.push(HeldObject::Retained(leaf));
    Ok(digest)
}

#[derive(Deserialize)]
struct PointSlots<'a> {
    #[serde(borrow)]
    status: &'a RawValue,
    #[serde(borrow)]
    schedule: &'a RawValue,
}

#[derive(Deserialize)]
struct CaptureView<'a> {
    #[serde(borrow)]
    requested_url: &'a str,
    body_bytes: usize,
    #[serde(borrow)]
    body_sha256: &'a str,
    #[serde(borrow)]
    body_base64: &'a str,
}

enum HeldObject {
    Inline {
        reference: AttachmentObjectReferenceV1,
        bytes: Vec<u8>,
    },
    Retained(RetainedCaptureLeafV1),
}

impl HeldObject {
    fn reference(&self) -> &AttachmentObjectReferenceV1 {
        match self {
            HeldObject::Inline { reference, .. } => reference,
            HeldObject::Retained(leaf) => &leaf.object,
        }
    }

    fn borrowed(&self) -> AttachmentContextObjectV1<'_> {
        match self {
            HeldObject::Inline { reference, bytes } => {
                AttachmentContextObjectV1::Inline { reference, bytes }
            }
            HeldObject::Retained(leaf) => AttachmentContextObjectV1::Retained(leaf),
        }
    }
}

struct PointMaterial {
    subject: RetainedDocumentSubjectReferenceV1,
    status_fact: AttachmentDigestV1,
    schedule_fact: AttachmentDigestV1,
    lineage: AttachmentDigestV1,
    schedule_lineage: AttachmentDigestV1,
    participation_root: AttachmentObjectReferenceV1,
    name: String,
    cohort: GradYear,
    school: String,
    day: ProfileDate,
}

type TargetMaterial = (
    AttachmentDigestV1,
    AttachmentDigestV1,
    AttachmentDigestV1,
    String,
    String,
    String,
);
fn capture_body(view: &CaptureView<'_>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if view.body_bytes == 0 || view.body_bytes > 512 * 1024 {
        return Err("genuine point capture body size out of bounds".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(view.body_base64)?;
    if bytes.len() != view.body_bytes {
        return Err("genuine point body length differs from declared size".into());
    }
    if object_reference(&bytes)?.digest.as_str() != view.body_sha256 {
        return Err("genuine point capture digest mismatch".into());
    }
    Ok(bytes)
}

fn build_point_material(
    snapshot: &census_store::StoreSnapshot<'_>,
    held: &mut Vec<HeldObject>,
    reference: &ParticipationEvidenceRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<PointMaterial, Box<dyn std::error::Error>> {
    let recorded = read_recorded_independent_participation(snapshot, reference, grant)?
        .ok_or("genuine point participation packet missing")?;
    let checked = recorded.checked();
    let document = match checked.ownership() {
        census_crawl::participation_evidence::SubjectOwnershipV1::DocumentOnly(document) => {
            document
        }
        _ => return Err("genuine point requires documentary ownership".into()),
    };
    if document != checked.subject() || checked.status_proof().locator() != document.locator() {
        return Err("genuine point documentary subject mismatch".into());
    }
    if checked.cohort() != GradYear::CO2027 {
        return Err("genuine point is not Class of 2027".into());
    }
    let root = recorded.root();
    let raw = snapshot
        .journal_payload_raw_bounded(
            root.schema.phase(),
            &root.key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("genuine point raw participation packet missing")?;
    let slots: PointSlots<'_> = serde_json::from_str(raw.get())?;
    let status: CaptureView<'_> = serde_json::from_str(slots.status.get())?;
    let schedule: CaptureView<'_> = serde_json::from_str(slots.schedule.get())?;
    let status_body = capture_body(&status)?;
    if status.requested_url != document.document_url()
        || status.body_sha256 != document.body_sha256()
        || status.body_sha256 != checked.status_proof().capture()
    {
        return Err("genuine point status capture differs from checked source".into());
    }
    if schedule.requested_url != "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26"
        || schedule.body_sha256 != checked.season_proof().capture()
    {
        return Err("genuine point schedule capture differs from checked source".into());
    }
    let proof = checked.status_proof();
    let rest = proof
        .locator()
        .strip_prefix("html_bytes:")
        .ok_or("genuine point locator prefix missing")?;
    let (start_text, end_text) = rest
        .split_once(':')
        .ok_or("genuine point locator span missing")?;
    let start: usize = start_text
        .parse()
        .map_err(|_| "genuine point locator start invalid")?;
    let end: usize = end_text
        .parse()
        .map_err(|_| "genuine point locator end invalid")?;
    let quote = status_body
        .get(start..end)
        .ok_or("genuine point locator range invalid")?;
    let status_reference = object_reference(slots.status.get().as_bytes())?;
    let status_object = status_reference.clone();
    let status_leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: status_reference,
    };
    let root_object = root.payload.clone();
    if root.schema != RetainedCaptureRootSchemaV1::ParticipationPacketV1
        || object_reference(raw.get().as_bytes())? != root_object
    {
        return Err("genuine point participation root payload differs from recorded source".into());
    }
    let root_leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::RootJson,
        object: root_object.clone(),
    };
    let body_reference = object_reference(&status_body)?;
    let body_object = body_reference.clone();
    let body_leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body_reference,
    };
    let schedule_reference = object_reference(slots.schedule.get().as_bytes())?;
    let schedule_object = schedule_reference.clone();
    let schedule_leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::ParticipationSchedule,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: schedule_reference,
    };
    let schedule_body = capture_body(&schedule)?;
    let schedule_body_reference = object_reference(&schedule_body)?;
    let schedule_body_object = schedule_body_reference.clone();
    let schedule_body_leaf = recorded.calendar_original_body().clone();
    if schedule_body_leaf.object != schedule_body_reference {
        return Err("genuine point schedule body descriptor differs from recorded source".into());
    }
    hold_retained(held, status_leaf, grant)?;
    hold_retained(held, root_leaf, grant)?;
    hold_retained(held, body_leaf, grant)?;
    hold_retained(held, schedule_leaf, grant)?;
    hold_retained(held, schedule_body_leaf, grant)?;
    let interpretation = format!(
        "milesplit_original_student_grade_dcc_event_day_v1\n{}\n{}",
        proof.capture(),
        proof.locator()
    );
    let interpretation_digest = hold_inline(held, interpretation.as_bytes().to_vec(), grant)?;
    if interpretation_digest.as_str() != proof.interpretation() {
        return Err("genuine point interpretation differs from checked source".into());
    }
    let subject_preimage = format!(
        "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
        document.document_namespace(),
        document.native_document_id(),
        document.document_url(),
        document.body_sha256(),
        document.locator()
    );
    let subject_digest = hold_inline(held, subject_preimage.into_bytes(), grant)?;
    if subject_digest.as_str() != document.reference() {
        return Err("genuine point subject digest differs from checked source".into());
    }
    let locator_digest = hold_inline(held, proof.locator().as_bytes().to_vec(), grant)?;
    let quote_digest = hold_inline(held, quote.to_vec(), grant)?;
    let source_context = (
        "genuine-point-original-context/v1",
        checked.reference(),
        checked.day(),
        checked.cohort(),
        checked.competition(),
        checked.venue(),
        checked.season_scope(),
        (
            proof.capture(),
            proof.interpretation(),
            proof.locator(),
            proof.acquired_at(),
        ),
        (
            checked.season_proof().capture(),
            checked.season_proof().interpretation(),
            checked.season_proof().locator(),
            checked.season_proof().acquired_at(),
        ),
        root,
        &status_object,
        &body_object,
        &schedule_object,
        &schedule_body_object,
    );
    let context_digest = hold_inline(held, serialized_bytes(&source_context)?, grant)?;
    let lineage = (
        "genuine-reviewed-publisher-family/v1",
        "mi.milesplit.com",
        document.document_url(),
        document.body_sha256(),
        status.requested_url,
        status.body_sha256,
        proof.acquired_at(),
    );
    let lineage_digest = hold_inline(held, serialized_bytes(&lineage)?, grant)?;
    let schedule_lineage = (
        "genuine-reviewed-dcc-schedule/v1",
        schedule.requested_url,
        schedule.body_sha256,
        checked.season_proof().capture(),
        checked.season_proof().interpretation(),
        checked.season_proof().locator(),
        checked.season_proof().acquired_at(),
        &schedule_body_object,
    );
    let schedule_lineage_digest = hold_inline(held, serialized_bytes(&schedule_lineage)?, grant)?;
    let schedule_fact = hold_inline(
        held,
        serialized_bytes(&(
            "genuine-point-dcc-schedule-fact/v1",
            checked.day(),
            checked.school_year(),
            checked.competition(),
            checked.venue(),
            checked.season_scope(),
            checked.season_proof().capture(),
            checked.season_proof().interpretation(),
            checked.season_proof().locator(),
            checked.season_proof().acquired_at(),
            &schedule_body_object,
        ))?,
        grant,
    )?;
    Ok(PointMaterial {
        subject: RetainedDocumentSubjectReferenceV1 {
            namespace: text(document.document_namespace())?,
            native_id: Some(text(document.native_document_id())?),
            capture: status_object.digest.clone(),
            body: body_object.digest.clone(),
            interpretation: interpretation_digest,
            locator: locator_digest,
            subject: subject_digest,
            context: context_digest,
            role: AttachmentSubjectRoleV1::AthleteStatus,
        },
        status_fact: quote_digest,
        schedule_fact,
        lineage: lineage_digest,
        schedule_lineage: schedule_lineage_digest,
        participation_root: root_object,
        name: document.name().to_owned(),
        cohort: checked.cohort(),
        school: document.school().to_owned(),
        day: checked.day().clone(),
    })
}

fn binding_matches_target(
    accepted: &census_store::AcceptedPublicIdentitySnapshot,
    candidate: &AthleteId,
    person: &census_domain::model::SourceIdentity,
    target: &census_domain::public_identity_history::PublicAthleteId,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<bool, Box<dyn std::error::Error>> {
    accepted.history_records().iter().try_fold(
        false,
        |found, record| -> Result<bool, Box<dyn std::error::Error>> {
            grant.charge_steps(1)?;
            let bindings = match &record.request {
                HistoryRequest::Establish { binding, .. }
                | HistoryRequest::Bind { binding, .. } => std::slice::from_ref(binding),
                HistoryRequest::EstablishMatchedGroup { bindings, .. } => bindings.as_slice(),
                _ => &[],
            };
            bindings.iter().try_fold(
                found,
                |found, binding| -> Result<bool, Box<dyn std::error::Error>> {
                    grant.charge_steps(1)?;
                    if binding.candidate != candidate.cast()
                        || binding.source.namespace != person.namespace
                        || binding.source.identifier != person.id
                    {
                        return Ok(found);
                    }
                    let current = matches!(
                        accepted.resolve_binding(&binding.id)?,
                        BindingResolution::Resolved {
                            identity: PublicResolution::Canonical(resolved),
                            superseded_by: None,
                            ..
                        } if resolved.public_id.as_str() == target.as_str()
                    );
                    Ok(found || current)
                },
            )
        },
    )
}

fn provider_for_candidate(
    snapshot: &census_store::StoreSnapshot<'_>,
    target_ref: &str,
    candidate: &AthleteId,
    target: &census_domain::public_identity_history::PublicAthleteId,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<RecordedProviderTargetV1, Box<dyn std::error::Error>> {
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("genuine point public identity authority absent")?;
    let parsed = SourceTargetPacketRefV1::parse(target_ref)?;
    let provider = read_recorded_provider_target(snapshot, &parsed, grant)?
        .ok_or("exact retained provider target packet missing")?;
    let person = provider.checked().profile().person();
    if person.namespace != SourceNamespace::MilesplitAthlete
        || person.id != "14570824"
        || !binding_matches_target(&accepted, candidate, person, target, grant)?
    {
        return Err("exact retained provider target binding missing".into());
    }
    Ok(provider)
}

fn build_target_material(
    snapshot: &census_store::StoreSnapshot<'_>,
    held: &mut Vec<HeldObject>,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    target_ref: &str,
    candidate: &AthleteId,
    point: &PointMaterial,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<TargetMaterial, Box<dyn std::error::Error>> {
    let provider = provider_for_candidate(
        snapshot,
        target_ref,
        candidate,
        &inspection.target().public_id,
        grant,
    )?;
    let checked = provider.checked().profile();
    if checked.name() != point.name
        || checked.cohort() != point.cohort
        || checked.school_name() != point.school
        || checked.school().namespace != SourceNamespace::MilesplitSchool
        || checked.school().id != "12439"
    {
        return Err("genuine point person, cohort or school differs from provider target".into());
    }
    let raw = snapshot
        .journal_payload_raw_bounded(
            provider.root().schema.phase(),
            &provider.root().key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("genuine point provider original payload missing")?;
    #[derive(Deserialize)]
    struct ProviderEnvelope<'a> {
        #[serde(borrow)]
        profile: &'a RawValue,
        #[serde(borrow)]
        linked_results: &'a RawValue,
    }
    let envelope: ProviderEnvelope<'_> = serde_json::from_str(raw.get())?;
    let profile_capture: CaptureView<'_> = serde_json::from_str(envelope.profile.get())?;
    let linked_capture: CaptureView<'_> = serde_json::from_str(envelope.linked_results.get())?;
    let profile_body = capture_body(&profile_capture)?;
    let linked_body = capture_body(&linked_capture)?;
    let profile_json = object_reference(envelope.profile.get().as_bytes())?;
    let linked_json = object_reference(envelope.linked_results.get().as_bytes())?;
    let profile_body_reference = object_reference(&profile_body)?;
    let linked_body_reference = object_reference(&linked_body)?;
    [
        RetainedCaptureLeafV1 {
            version: 1,
            root: provider.root().clone(),
            slot: RetainedCaptureSlotV1::ProviderProfile,
            kind: RetainedCaptureLeafKindV1::CaptureJson,
            object: profile_json.clone(),
        },
        RetainedCaptureLeafV1 {
            version: 1,
            root: provider.root().clone(),
            slot: RetainedCaptureSlotV1::ProviderProfile,
            kind: RetainedCaptureLeafKindV1::OriginalBody,
            object: profile_body_reference.clone(),
        },
        RetainedCaptureLeafV1 {
            version: 1,
            root: provider.root().clone(),
            slot: RetainedCaptureSlotV1::ProviderLinkedResults,
            kind: RetainedCaptureLeafKindV1::CaptureJson,
            object: linked_json.clone(),
        },
        RetainedCaptureLeafV1 {
            version: 1,
            root: provider.root().clone(),
            slot: RetainedCaptureSlotV1::ProviderLinkedResults,
            kind: RetainedCaptureLeafKindV1::OriginalBody,
            object: linked_body_reference.clone(),
        },
    ]
    .into_iter()
    .try_for_each(|leaf| hold_retained(held, leaf, grant).map(|_| ()))?;
    let row = provider
        .checked()
        .categories()
        .rows()
        .iter()
        .filter(|row| row.meet_day().as_str() == point.day.as_str())
        .try_fold(None, |found, row| {
            grant.charge_steps(1)?;
            if found.is_some() {
                return Err("multiple provider target rows match the participation day".into());
            }
            Ok::<_, Box<dyn std::error::Error>>(Some(row))
        })?
        .ok_or("provider target has no exact participation-day row")?;
    if !checked
        .proofs()
        .iter()
        .any(|proof| proof.capture() == profile_capture.body_sha256)
        || row.proof().capture() != linked_capture.body_sha256
    {
        return Err("provider target proof does not match retained original captures".into());
    }
    let target_context = (
        "reviewed-original-person-specific-target/v1",
        (
            inspection.accepted_target_witness(),
            checked.person(),
            checked.name(),
            checked.school(),
            checked.school_name(),
            checked.cohort(),
            provider.checked().reference().as_str(),
            provider.root(),
            &profile_json,
            &profile_body_reference,
            &linked_json,
            &linked_body_reference,
        ),
        (
            row.meet_id(),
            row.result_document_id(),
            row.result_document_url(),
            row.meet_day(),
            row.event_code(),
            row.mark(),
            row.proof().capture(),
            row.proof().locator(),
            row.proof().interpretation(),
        ),
    );
    let subject = hold_inline(held, serialized_bytes(&target_context)?, grant)?;
    let target_fact_preimage = (
        "genuine-original-source-bound-person/v1",
        provider.checked().reference().as_str(),
        checked.person(),
        checked.name(),
        checked.school(),
        checked.school_name(),
        checked.cohort(),
        row.meet_id(),
        row.result_document_id(),
        row.result_document_url(),
        row.meet_day(),
        row.event_code(),
        row.mark(),
        row.proof().capture(),
        row.proof().locator(),
        row.proof().interpretation(),
    );
    let fact = hold_inline(held, serialized_bytes(&target_fact_preimage)?, grant)?;
    let lineage = hold_inline(
        held,
        serialized_bytes(&(
            "genuine-reviewed-publisher-family/v1",
            "mi.milesplit.com",
            provider.root(),
            &profile_json,
            &profile_body_reference,
            &linked_json,
            &linked_body_reference,
        ))?,
        grant,
    )?;
    Ok((
        subject,
        fact,
        lineage,
        checked.person().id.clone(),
        checked.name().to_owned(),
        checked.school_name().to_owned(),
    ))
}

fn object_is_existing(
    reference: &AttachmentObjectReferenceV1,
    existing: &[AttachmentObjectReferenceV1],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<bool, Box<dyn std::error::Error>> {
    for prior in existing {
        grant.charge_steps(1)?;
        if prior == reference {
            return Ok(true);
        }
    }
    Ok(false)
}

fn build_reviewed_context(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    held: &mut Vec<HeldObject>,
    existing: &[AttachmentObjectReferenceV1],
    material: &PointMaterial,
    target: &TargetMaterial,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ReviewedAttachmentContextV1, Box<dyn std::error::Error>> {
    let mut facts = vec![
        ReviewedAttachmentFactV1 {
            fact: material.status_fact.clone(),
            subject: material.subject.subject.clone(),
            publisher_lineage: material.lineage.clone(),
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Original retained status bytes and checked locator for the exact participation day")?,
        },
        ReviewedAttachmentFactV1 {
            fact: material.schedule_fact.clone(),
            subject: material.subject.subject.clone(),
            publisher_lineage: material.schedule_lineage.clone(),
            disposition: AttachmentFactDispositionV1::ContextOnly,
            rationale: text("Original retained DCC schedule capture and checked season scope; no inferred participation")?,
        },
        ReviewedAttachmentFactV1 {
            fact: target.1.clone(),
            subject: target.0.clone(),
            publisher_lineage: target.2.clone(),
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Exact retained provider person, cohort, school and participation-day linked row")?,
        },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let pairs = vec![AttachmentCorroborationPairV1 {
        subject_fact: material.status_fact.clone(),
        accepted_target_fact: target.1.clone(),
        basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
        explanation: text("Original same-publisher status document explicitly cross-references the exact provider-bound person, cohort, school, event day and mark")?,
    }];
    let scope_preimage = (
        "genuine-original-documentary-complete-scope/v1",
        &material.subject,
        &facts,
        &pairs,
        &target.0,
        inspection.target(),
        inspection.accepted_target_witness(),
        inspection.public_head(),
        inspection.inventory(),
        material.name.as_str(),
        material.cohort,
        material.school.as_str(),
        material.day.as_str(),
        &target.3,
        &target.4,
        &target.5,
    );
    let scope_bytes = serialized_bytes(&scope_preimage)?;
    let scope_reference = object_reference(&scope_bytes)?;
    let scope_digest = hold_inline(held, scope_bytes, grant)?;
    let mut references: Vec<AttachmentObjectReferenceV1> = held
        .iter()
        .map(|object| object.reference().clone())
        .collect();
    references.extend(existing.iter().cloned());
    references.push(scope_reference);
    references.extend(inspection.issued_objects().cloned());
    references.sort();
    references.dedup_by(|left, right| {
        if left.digest == right.digest && left.bytes != right.bytes {
            false
        } else {
            left.digest == right.digest
        }
    });
    if references.windows(2).any(|pair| {
        matches!(pair, [left, right] if left.digest == right.digest && left.bytes != right.bytes)
    }) {
        return Err("genuine point object digest has conflicting lengths".into());
    }
    Ok(ReviewedAttachmentContextV1 {
        objects: list(references)?,
        facts: list(facts)?,
        corroboration: list(pairs)?,
        complete_scope_records: list(vec![scope_digest])?,
        accepted_target_subjects: list(vec![target.0.clone()])?,
        accepted_target_witness: inspection.accepted_target_witness().clone(),
    })
}
fn build_correction_context(
    previous: &ReviewedAttachmentContextV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    held: &[HeldObject],
    material: &PointMaterial,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ReviewedAttachmentContextV1, Box<dyn std::error::Error>> {
    let mut facts = previous.facts.as_slice().to_vec();
    if !facts.iter().any(|fact| fact.fact == material.schedule_fact) {
        grant.charge_steps(1)?;
        facts.push(ReviewedAttachmentFactV1 {
            fact: material.schedule_fact.clone(),
            subject: material.subject.subject.clone(),
            publisher_lineage: material.schedule_lineage.clone(),
            disposition: AttachmentFactDispositionV1::ContextOnly,
            rationale: text(
                "Correction adds the retained DCC schedule lineage; the prior context-only schedule metadata used the wrong publisher lineage",
            )?,
        });
    }
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let mut objects = previous.objects.as_slice().to_vec();
    objects.push(material.participation_root.clone());
    [
        material.schedule_fact.clone(),
        material.schedule_lineage.clone(),
    ]
    .into_iter()
    .try_for_each(|digest| {
        let reference = held
            .iter()
            .find(|object| object.reference().digest == digest)
            .ok_or("genuine point correction schedule object missing")?
            .reference()
            .clone();
        objects.push(reference);
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    objects.extend(inspection.issued_objects().cloned());
    objects.sort();
    objects.dedup_by(|left, right| {
        if left.digest == right.digest && left.bytes != right.bytes {
            false
        } else {
            left.digest == right.digest
        }
    });
    if objects.windows(2).any(|pair| {
        matches!(pair, [left, right] if left.digest == right.digest && left.bytes != right.bytes)
    }) {
        return Err("genuine point correction object digest has conflicting lengths".into());
    }
    Ok(ReviewedAttachmentContextV1 {
        objects: list(objects)?,
        facts: list(facts)?,
        corroboration: previous.corroboration.clone(),
        complete_scope_records: previous.complete_scope_records.clone(),
        accepted_target_subjects: previous.accepted_target_subjects.clone(),
        accepted_target_witness: previous.accepted_target_witness.clone(),
    })
}

fn retain_artifact(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if !std::fs::symlink_metadata(path)?.file_type().is_file() {
                return Err("point artifact is not a regular file".into());
            }
            let mut file = std::fs::File::open(path)?;
            if file.metadata()?.len() != u64::try_from(bytes.len())? {
                return Err("point artifact differs from original length".into());
            }
            let mut buffer = [0_u8; 4096];
            bytes
                .chunks(4096)
                .try_for_each(|chunk| -> Result<(), Box<dyn std::error::Error>> {
                    let target = buffer
                        .get_mut(..chunk.len())
                        .ok_or("point artifact comparison buffer")?;
                    file.read_exact(target)?;
                    if target != chunk {
                        return Err("point artifact differs from original bytes".into());
                    }
                    Ok(())
                })?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn read_retained_bounded(
    path: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() > max_bytes {
        return Err("retained file exceeds bounded read limit".into());
    }
    let len: usize = u64::try_into(metadata.len())?;
    let mut content = Vec::new();
    content.try_reserve_exact(len)?;
    content.resize(len, 0);
    file.read_exact(&mut content)?;
    match file.read(&mut [0])? {
        0 => {}
        _ => return Err("retained file grew during read".into()),
    }
    Ok(content)
}

fn apply_point(
    store: &Store,
    prepared: census_store::PreparedEvidenceSubjectAttachmentV1<'_>,
    artifacts: &Path,
    correction: bool,
) -> Result<census_store::Receipt, Box<dyn std::error::Error>> {
    let replay = prepared.is_replay();
    let prefix = if correction {
        "point-correction"
    } else {
        "point"
    };
    let approval_path = artifacts.join(format!("{prefix}-approval.bin"));
    let digest_path = artifacts.join(format!("{prefix}-approval.digest"));
    if replay {
        let retained_approval = read_retained_bounded(&approval_path, 1024 * 1024)?;
        if retained_approval != prepared.approval_bytes() {
            return Err(
                "retained original approval differs from ReplayOnly prepared approval".into(),
            );
        }
        let retained_digest_bytes = read_retained_bounded(&digest_path, 256)?;
        let retained_digest =
            AttachmentDigestV1::parse(std::str::from_utf8(&retained_digest_bytes)?)?;
        if &retained_digest != prepared.approval_digest() {
            return Err(
                "retained original approval digest differs from ReplayOnly prepared digest".into(),
            );
        }
        let receipt = match store.apply_evidence_subject_attachment_v1(
            prepared,
            &retained_approval,
            &retained_digest,
        )? {
            Application::Repeated(receipt) => receipt,
            other => return Err(format!("replay expected Repeated, got {other:?}").into()),
        };
        let retained_receipt_path = artifacts.join(format!("{prefix}-receipt.json"));
        let retained_receipt_bytes = read_retained_bounded(&retained_receipt_path, 64 * 1024)?;
        let retained_receipt: census_store::Receipt =
            serde_json::from_slice(&retained_receipt_bytes)?;
        if retained_receipt != receipt {
            return Err("replay persisted receipt differs from retained original receipt".into());
        }
        if store.receipt(&receipt.operation)?.as_ref() != Some(&receipt) {
            return Err("replay receipt differs from persisted receipt".into());
        }
        return Ok(receipt);
    }
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    retain_artifact(&approval_path, &approval)?;
    retain_artifact(&digest_path, digest.as_str().as_bytes())?;
    retain_artifact(
        &artifacts.join(format!("{prefix}-approval.sha256")),
        object_reference(&approval)?.digest.as_str().as_bytes(),
    )?;
    retain_artifact(
        &artifacts.join(format!("{prefix}-command.json")),
        &serialized_bytes(prepared.command())?,
    )?;
    retain_artifact(
        &artifacts.join(format!("{prefix}-operator.txt")),
        b"Unauthenticated local operator provenance. Source-derived proposal; no authenticated human review claim.\n",
    )?;
    let receipt = match store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)? {
        Application::Written(receipt) | Application::Repeated(receipt) => receipt,
    };
    retain_artifact(
        &artifacts.join(format!("{prefix}-receipt.json")),
        &serialized_bytes(&receipt)?,
    )?;
    if store.receipt(&receipt.operation)?.as_ref() != Some(&receipt) {
        return Err("committed point receipt differs from persisted receipt".into());
    }
    Ok(receipt)
}

fn verify_visible(
    store: &Store,
    public: &census_domain::public_identity_history::PublicAthleteId,
    subject: &RetainedDocumentSubjectReferenceV1,
    operation: &AttachmentTextV1,
    artifacts: &Path,
    correction: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let snapshot = store.snapshot();
    let mut read_grant = AttachmentReplayGrantV1::new();
    let current =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut read_grant)?;
    let active = current
        .bind_exact_subject(subject, &mut read_grant)?
        .ok_or("genuine point is not visible in the current v4 selector")?;
    active.require_original_grant(&mut read_grant)?;
    if active.target().public_id.as_str() != public.as_str()
        || active.record().command.operation.as_str() != operation.as_str()
    {
        return Err("visible genuine point target or operation differs".into());
    }
    let record_name = if correction {
        "point-correction-record.json"
    } else {
        "point-record.json"
    };
    retain_artifact(
        &artifacts.join(record_name),
        &serialized_bytes(active.record())?,
    )?;
    Ok(format!(
        "{}:{}",
        operation.as_str(),
        active.record().record_digest.as_str()
    ))
}

pub(super) fn ensure_point(
    store: &Store,
    candidate: &AthleteId,
    participation: &str,
    target_ref: &str,
    artifacts: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let participation = ParticipationEvidenceRefV1::parse(participation)?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("genuine point store has no accepted public identity authority")?;
    let budgeted_candidate = candidate.cast();
    let public = match accepted.resolve_candidate(&budgeted_candidate)? {
        PublicResolution::Canonical(row) => row.public_id.clone(),
        other => {
            return Err(format!(
                "genuine point candidate has no canonical public resolution: {other:?}"
            )
            .into())
        }
    };
    let operation = text("genuine-co2027-owen-may9-point")?;
    let correction_operation = text("genuine-co2027-owen-may9-point-root-custody")?;
    let mut source_grant = AttachmentReplayGrantV1::new();
    let mut held = Vec::new();
    let material = build_point_material(&snapshot, &mut held, &participation, &mut source_grant)?;
    let records = snapshot.read_evidence_subject_attachment_references_v1(&mut source_grant)?;
    let existing_objects: Vec<AttachmentObjectReferenceV1> = records
        .records()
        .iter()
        .flat_map(|record| {
            record
                .command
                .reviewed_context
                .objects
                .as_slice()
                .iter()
                .cloned()
        })
        .collect();
    let original = records
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == operation.as_str());
    let correction = records
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == correction_operation.as_str());
    if original.is_none() && correction.is_some() {
        return Err("genuine point root custody correction lacks its original record".into());
    }
    let Some(original) = original else {
        let inspection = store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: public.clone(),
                prior: AttachmentPredecessorV1::First,
            },
        )?;
        let target = build_target_material(
            &snapshot,
            &mut held,
            &inspection,
            target_ref,
            candidate,
            &material,
            &mut source_grant,
        )?;
        let reviewed_context = build_reviewed_context(
            &inspection,
            &mut held,
            &existing_objects,
            &material,
            &target,
            &mut source_grant,
        )?;
        let command = RetainedEvidenceSubjectAttachmentCommandV1 {
            version: 1,
            policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
            store: inspection.store_identity().clone(),
            operation: operation.clone(),
            expected_public_head: inspection.public_head().clone(),
            expected_evidence_generation: inspection.evidence_generation(),
            expected_derived_generation: inspection.derived_generation(),
            expected_attachment_inventory: inspection.inventory().clone(),
            target: inspection.target().clone(),
            subjects: list(vec![material.subject.clone()])?,
            action: inspection.action().clone(),
            reviewed_context,
            actor: AttachmentReviewActorV1::LocalOperator,
            reviewed_at: Utc::now(),
            rationale: text("Unauthenticated local operator provenance for original May9 source review; no accepted facts injected")?,
        };
        let mut context_objects = Vec::new();
        for object in &held {
            if !object_is_existing(object.reference(), &existing_objects, &mut source_grant)? {
                context_objects.push(object.borrowed());
            }
        }
        let prepared = store.prepare_evidence_subject_attachment_v1(
            command,
            EvidenceSubjectAttachmentPreparationV1::New {
                inspection,
                context: &context_objects,
            },
        )?;
        apply_point(store, prepared, artifacts, false)?;
        return verify_visible(
            store,
            &public,
            &material.subject,
            &operation,
            artifacts,
            false,
        );
    };
    if original.command.target.public_id.as_str() != public.as_str()
        || original.command.target.canonical_candidate != budgeted_candidate
        || original.command.subjects.as_slice() != std::slice::from_ref(&material.subject)
    {
        return Err("existing genuine point operation belongs to another target or source".into());
    }
    let original_reference = original.reference();
    let prepared = store.prepare_evidence_subject_attachment_v1(
        original.command.clone(),
        EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
    )?;
    if !prepared.is_replay() {
        return Err("existing genuine point did not prepare as exact replay".into());
    }
    apply_point(store, prepared, artifacts, false)?;
    if let Some(correction) = correction {
        if correction.command.target.public_id.as_str() != public.as_str()
            || correction.command.target.canonical_candidate != budgeted_candidate
            || correction.command.subjects.as_slice() != std::slice::from_ref(&material.subject)
            || !matches!(
                &correction.command.action,
                AttachmentActionV1::Attach {
                    prior: AttachmentPredecessorV1::After(prior)
                } if prior == &original_reference
            )
            || !correction
                .command
                .reviewed_context
                .objects
                .as_slice()
                .iter()
                .any(|object| object == &material.participation_root)
        {
            return Err("existing genuine point correction belongs to another source".into());
        }
        let inspection = store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: public.clone(),
                prior: AttachmentPredecessorV1::After(correction.reference()),
            },
        )?;
        let target = build_target_material(
            &snapshot,
            &mut held,
            &inspection,
            target_ref,
            candidate,
            &material,
            &mut source_grant,
        )?;
        if !correction
            .command
            .reviewed_context
            .accepted_target_subjects
            .as_slice()
            .contains(&target.0)
            || !correction
                .command
                .reviewed_context
                .facts
                .as_slice()
                .iter()
                .any(|fact| {
                    fact.fact == target.1
                        && fact.subject == target.0
                        && fact.publisher_lineage == target.2
                })
        {
            return Err("genuine point correction source differs from retained target".into());
        }
        let prepared = store.prepare_evidence_subject_attachment_v1(
            correction.command.clone(),
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        )?;
        if !prepared.is_replay() {
            return Err("existing genuine point correction did not prepare as exact replay".into());
        }
        apply_point(store, prepared, artifacts, true)?;
        return verify_visible(
            store,
            &public,
            &material.subject,
            &correction_operation,
            artifacts,
            true,
        );
    }
    let original_result = verify_visible(
        store,
        &public,
        &material.subject,
        &operation,
        artifacts,
        false,
    )?;
    if original
        .command
        .reviewed_context
        .objects
        .as_slice()
        .iter()
        .any(|object| object == &material.participation_root)
    {
        return Ok(original_result);
    }
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: public.clone(),
            prior: AttachmentPredecessorV1::After(original_reference.clone()),
        })?;
    build_target_material(
        &snapshot,
        &mut held,
        &inspection,
        target_ref,
        candidate,
        &material,
        &mut source_grant,
    )?;
    let reviewed_context = build_correction_context(
        &original.command.reviewed_context,
        &inspection,
        &held,
        &material,
        &mut source_grant,
    )?;
    let command = RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1,
        policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: correction_operation.clone(),
        expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(),
        expected_derived_generation: inspection.derived_generation(),
        expected_attachment_inventory: inspection.inventory().clone(),
        target: inspection.target().clone(),
        subjects: list(vec![material.subject.clone()])?,
        action: inspection.action().clone(),
        reviewed_context,
        actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: Utc::now(),
        rationale: text("Unauthenticated local operator correction adds retained participation root custody and corrects the DCC schedule lineage without rewriting the original review")?,
    };
    let mut context_objects = Vec::new();
    for object in &held {
        if !object_is_existing(object.reference(), &existing_objects, &mut source_grant)? {
            context_objects.push(object.borrowed());
        }
    }
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &context_objects,
        },
    )?;
    apply_point(store, prepared, artifacts, true)?;
    verify_visible(
        store,
        &public,
        &material.subject,
        &correction_operation,
        artifacts,
        true,
    )
}
