use anyhow::{ensure, Context};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_crawl::milesplit::{parse_published_time, SourceRowLocator};
use census_crawl::participation_evidence::provider_target::{
    CheckedLinkedEventLabelV1, SourceTargetPacketRefV1, SourceTargetPacketV1,
};
use census_crawl::participation_evidence::recorded::{
    read_recorded_html_result_capture, read_recorded_provider_target, RecordedProviderTargetV1,
};
use census_crawl::participation_evidence::result_ownership::{
    CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1,
};
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentCorroborationBasisV1, AttachmentCorroborationPairV1,
    AttachmentDigestV1, AttachmentFactDispositionV1, AttachmentListV1, AttachmentObjectReferenceV1,
    AttachmentPredecessorV1, AttachmentRecordReferenceV1, AttachmentReplayGrantV1,
    AttachmentReviewActorV1, AttachmentSubjectRoleV1, AttachmentTextV1,
    EvidenceSubjectAttachmentError, RetainedDocumentSubjectReferenceV1,
    RetainedEvidenceSubjectAttachmentCommandV1, ReviewedAttachmentContextV1,
    ReviewedAttachmentFactV1, ReviewedReferenceRecordV1, EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1,
};
use census_domain::model::{write_serialized, AthleteCandidateId, AthleteId, EventKind, Mark};
use census_domain::public_identity_history::{BindingResolution, HistoryRequest, PublicResolution};
use census_store::{
    AcceptedPublicIdentitySnapshot, Application, AttachmentContextObjectV1,
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, PreparedEvidenceSubjectAttachmentV1,
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
    Store, StoreSnapshot,
};
use chrono::Utc;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

const OPERATION: &str = "genuine-owen-result-attachment";
const EVENT_CONTEXT_OPERATION: &str = "genuine-owen-checked-event-context";

fn reference(bytes: &[u8]) -> anyhow::Result<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

fn reserve<T>(
    values: &mut Vec<T>,
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<()> {
    grant.charge_bytes(
        count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?,
    )?;
    grant.charge_steps(count)?;
    values
        .try_reserve_exact(count)
        .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
    Ok(())
}

struct Encoding {
    bytes: usize,
}

impl Write for Encoding {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let needed = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other(EvidenceSubjectAttachmentError::Overflow))?;
        let available =
            census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes;
        if needed > available {
            return Err(std::io::Error::other(
                EvidenceSubjectAttachmentError::Capacity {
                    resource: "source context bytes",
                    needed,
                    available,
                },
            ));
        }
        self.bytes = needed;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encode(value: &impl Serialize, grant: &mut AttachmentReplayGrantV1) -> anyhow::Result<Vec<u8>> {
    grant.charge_steps(1)?;
    let mut measured = Encoding { bytes: 0 };
    write_serialized(&mut measured, value)?;
    let mut bytes = Vec::new();
    reserve(&mut bytes, measured.bytes, grant)?;
    write_serialized(&mut bytes, value)?;
    ensure!(bytes.len() == measured.bytes, "source context size changed");
    Ok(bytes)
}

fn text(value: &str) -> anyhow::Result<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

fn list<T, const N: usize>(
    values: [T; N],
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<AttachmentListV1<T>> {
    let mut result = Vec::new();
    reserve(&mut result, N, grant)?;
    result.extend(values);
    Ok(AttachmentListV1::new(result)?)
}

#[derive(Default)]
struct Objects {
    inline: Vec<(AttachmentObjectReferenceV1, Vec<u8>)>,
    retained: Vec<RetainedCaptureLeafV1>,
}

impl Objects {
    fn insert(
        &mut self,
        bytes: Vec<u8>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<AttachmentDigestV1> {
        let object = reference(&bytes)?;
        grant.charge_steps(self.inline.len())?;
        if let Some((held, original)) = self
            .inline
            .iter()
            .find(|(held, _)| held.digest == object.digest)
        {
            ensure!(
                held == &object && original == &bytes,
                "inline object redefined"
            );
            return Ok(object.digest);
        }
        reserve(&mut self.inline, 1, grant)?;
        grant.charge_bytes(64)?;
        self.inline.push((object.clone(), bytes));
        Ok(object.digest)
    }

    fn bytes(
        &mut self,
        bytes: &[u8],
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<AttachmentDigestV1> {
        let mut owned = Vec::new();
        reserve(&mut owned, bytes.len(), grant)?;
        owned.extend_from_slice(bytes);
        self.insert(owned, grant)
    }

    fn json(
        &mut self,
        value: &impl Serialize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<AttachmentDigestV1> {
        let bytes = encode(value, grant)?;
        self.insert(bytes, grant)
    }

    fn leaf(
        &mut self,
        root: &RetainedCaptureRootV1,
        slot: RetainedCaptureSlotV1,
        kind: RetainedCaptureLeafKindV1,
        object: AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<AttachmentDigestV1> {
        if let Some(existing) = self
            .retained
            .iter()
            .find(|leaf| leaf.root == *root && leaf.slot == slot && leaf.kind == kind)
        {
            grant.charge_steps(self.retained.len())?;
            ensure!(existing.object == object, "retained object redefined");
            return Ok(object.digest);
        }
        reserve(&mut self.retained, 1, grant)?;
        grant.charge_bytes(
            root.key
                .len()
                .checked_add(192)
                .ok_or(EvidenceSubjectAttachmentError::Overflow)?,
        )?;
        let digest = object.digest.clone();
        self.retained.push(RetainedCaptureLeafV1 {
            version: 1,
            root: root.clone(),
            slot,
            kind,
            object,
        });
        Ok(digest)
    }

    fn references(
        &self,
        inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<AttachmentListV1<AttachmentObjectReferenceV1>> {
        let mut result = Vec::new();
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .and_then(|n| n.checked_add(inspection.issued_objects().count()))
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        reserve(&mut result, count, grant)?;
        grant.charge_bytes(
            count
                .checked_mul(64)
                .ok_or(EvidenceSubjectAttachmentError::Overflow)?,
        )?;
        result.extend(
            self.inline
                .iter()
                .map(|(object, _)| object)
                .chain(self.retained.iter().map(|leaf| &leaf.object))
                .chain(inspection.issued_objects())
                .cloned(),
        );
        result.sort();
        result.dedup();
        ensure!(
            result
                .windows(2)
                .all(|pair| matches!(pair, [left, right] if left.digest != right.digest)),
            "object digest has conflicting lengths"
        );
        Ok(AttachmentListV1::new(result)?)
    }

    fn borrowed(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<Vec<AttachmentContextObjectV1<'_>>> {
        let mut result = Vec::new();
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        reserve(&mut result, count, grant)?;
        result.extend(
            self.inline
                .iter()
                .map(|(reference, bytes)| AttachmentContextObjectV1::Inline { reference, bytes })
                .chain(
                    self.retained
                        .iter()
                        .map(AttachmentContextObjectV1::Retained),
                ),
        );
        Ok(result)
    }
    fn borrowed_excluding(
        &self,
        existing: &[AttachmentObjectReferenceV1],
        grant: &mut AttachmentReplayGrantV1,
    ) -> anyhow::Result<Vec<AttachmentContextObjectV1<'_>>> {
        let mut result = Vec::new();
        let count = self
            .inline
            .iter()
            .filter(|(reference, _)| !existing.iter().any(|held| held.digest == reference.digest))
            .count()
            .checked_add(
                self.retained
                    .iter()
                    .filter(|leaf| {
                        !existing
                            .iter()
                            .any(|held| held.digest == leaf.object.digest)
                    })
                    .count(),
            )
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
        reserve(&mut result, count, grant)?;
        result.extend(
            self.inline
                .iter()
                .filter(|(reference, _)| {
                    !existing.iter().any(|held| held.digest == reference.digest)
                })
                .map(|(reference, bytes)| AttachmentContextObjectV1::Inline { reference, bytes })
                .chain(
                    self.retained
                        .iter()
                        .filter(|leaf| {
                            !existing
                                .iter()
                                .any(|held| held.digest == leaf.object.digest)
                        })
                        .map(AttachmentContextObjectV1::Retained),
                ),
        );
        Ok(result)
    }
}

fn row_subject(
    document: &CheckedRetainedResultDocumentV1<'_>,
    root: &RetainedCaptureRootV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<(RetainedDocumentSubjectReferenceV1, AttachmentDigestV1)> {
    let row = document.row(0).context("selected original row absent")?;
    let source = row.subject()?;
    let capture = objects.leaf(
        root,
        RetainedCaptureSlotV1::SelfCapture,
        RetainedCaptureLeafKindV1::CaptureJson,
        root.payload.clone(),
        grant,
    )?;
    let body = objects.leaf(
        root,
        RetainedCaptureSlotV1::SelfCapture,
        RetainedCaptureLeafKindV1::OriginalBody,
        document.capture().original_body_reference()?,
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
    ensure!(
        interpretation.as_str() == row.reference(),
        "original interpretation differs"
    );
    let page = document.parsed_page();
    let context = objects.json(
        &(
            "retained-milesplit-html-result-row/v1",
            document.capture(),
            &page.meet,
            &page.skipped,
            &page.grade_issues,
            document.published_start(),
            document.published_end(),
        ),
        grant,
    )?;
    ensure!(
        context.as_str() == document.context_reference() && context.as_str() == root.key,
        "original context differs"
    );
    let fields = [
        "document_subject_v1",
        source.document_namespace(),
        source.native_document_id(),
        source.document_url(),
        source.body_sha256(),
        source.locator(),
    ];
    let mut preimage = Vec::new();
    let size = fields.iter().try_fold(5usize, |n, value| {
        n.checked_add(value.len())
            .ok_or(EvidenceSubjectAttachmentError::Overflow)
    })?;
    reserve(&mut preimage, size, grant)?;
    fields.iter().enumerate().for_each(|(index, field)| {
        if index != 0 {
            preimage.push(b'\n');
        }
        preimage.extend_from_slice(field.as_bytes());
    });
    let subject = objects.insert(preimage, grant)?;
    ensure!(
        subject.as_str() == source.reference(),
        "original document subject differs"
    );
    let locator = objects.bytes(source.locator().as_bytes(), grant)?;
    let fact = objects.bytes(row.original_row()?.as_bytes(), grant)?;
    ensure!(
        fact.as_str() == row.original_row_digest(),
        "original row bytes differ"
    );
    grant.charge_bytes(
        source
            .document_namespace()
            .len()
            .checked_add(source.native_document_id().len())
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?,
    )?;
    Ok((
        RetainedDocumentSubjectReferenceV1 {
            namespace: text(source.document_namespace())?,
            native_id: Some(text(source.native_document_id())?),
            capture,
            body,
            interpretation,
            locator,
            subject,
            context,
            role: AttachmentSubjectRoleV1::ResultRow,
        },
        fact,
    ))
}

fn provider_target(
    snapshot: &StoreSnapshot<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    candidate: &AthleteCandidateId,
    target_ref: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<RecordedProviderTargetV1> {
    grant.charge_bytes(target_ref.len())?;
    let reference = SourceTargetPacketRefV1::parse(target_ref)?;
    let packet = read_recorded_provider_target(snapshot, &reference, grant)?
        .context("exact retained provider packet absent")?;
    let public = match accepted.resolve_candidate(candidate)? {
        PublicResolution::Canonical(value) => value.public_id,
        other => anyhow::bail!("candidate has no canonical public resolution: {other:?}"),
    };
    let person = packet.checked().profile().person();
    let bound = accepted.history_records().iter().try_fold(false, |bound, record| -> anyhow::Result<bool> {
        grant.charge_steps(1)?;
        let bindings = match &record.request {
            HistoryRequest::Establish { binding, .. } | HistoryRequest::Bind { binding, .. } => std::slice::from_ref(binding),
            HistoryRequest::EstablishMatchedGroup { bindings, .. } => bindings.as_slice(),
            _ => &[],
        };
        bindings.iter().try_fold(bound, |bound, binding| -> anyhow::Result<bool> {
            grant.charge_steps(1)?;
            if &binding.candidate != candidate || binding.source.namespace != person.namespace || binding.source.identifier != person.id { return Ok(bound); }
            let current = matches!(&accepted.resolve_binding(&binding.id)?, BindingResolution::Resolved { identity: PublicResolution::Canonical(target), superseded_by: None, .. } if target.public_id == public);
            Ok(bound || current)
        })
    })?;
    ensure!(
        bound,
        "exact retained provider person does not bind the accepted candidate"
    );
    Ok(packet)
}

fn original_link(
    raw: &serde_json::value::RawValue,
    capture: &census_crawl::participation_evidence::RetainedHtmlCaptureV1,
    locator: &str,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<AttachmentDigestV1> {
    grant.charge_bytes(raw.get().len())?;
    let fields: std::collections::BTreeMap<&str, &serde_json::value::RawValue> =
        serde_json::from_str(raw.get())?;
    let encoded: &str = serde_json::from_str(
        fields
            .get("body_base64")
            .context("original linked body absent")?
            .get(),
    )?;
    let mut body = Vec::new();
    reserve(
        &mut body,
        base64::decoded_len_estimate(encoded.len()),
        grant,
    )?;
    body.resize(body.capacity(), 0);
    let length = STANDARD.decode_slice(encoded, &mut body)?;
    body.truncate(length);
    ensure!(
        reference(&body)? == capture.original_body_reference()?,
        "original linked body differs"
    );
    let (start, end) = locator
        .strip_prefix("html_bytes:")
        .and_then(|value| value.split_once(':'))
        .context("linked proof locator is not an original HTML span")?;
    let span = std::str::from_utf8(&body)?
        .get(start.parse::<usize>()?..end.parse::<usize>()?)
        .context("linked proof span is outside original UTF-8 body")?;
    objects.bytes(span.as_bytes(), grant)
}

fn target_context(
    snapshot: &StoreSnapshot<'_>,
    provider: &RecordedProviderTargetV1,
    document: &CheckedRetainedResultDocumentV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<(AttachmentDigestV1, AttachmentDigestV1)> {
    let profile = provider.checked().profile();
    let row = document.row(0).context("selected original row absent")?;
    ensure!(
        row.event().kind == EventKind::Track400m,
        "genuine qualification requires the checked original 400m row"
    );
    ensure!(
        row.parsed_row().name == profile.name() && row.parsed_row().school == profile.school_name(),
        "original row name or school differs from source-owned person"
    );
    let year = document.parsed_page().school_year;
    let grade = row
        .parsed_row()
        .grade
        .context("original row grade unavailable")?;
    ensure!(
        census_domain::model::GradYear::of(grade, year) == Some(profile.cohort()),
        "original row grade contradicts provider graduation cohort"
    );
    let linked = provider
        .checked()
        .categories()
        .rows()
        .iter()
        .try_fold(None, |matched, linked| -> anyhow::Result<_> {
            grant.charge_steps(1)?;
            if linked.event_code() != "56"
                || linked.meet_id() != document.meet_id()
                || linked.result_document_id() != document.result_document_id()
                || linked.meet_day().as_str() != document.published_start()
            {
                return Ok(matched);
            }
            let (time, timing) = parse_published_time(linked.mark())
                .context("provider linked mark is not a checked time")?;
            if row.parsed_row().mark != Mark::TimeSeconds(time)
                || timing.is_some_and(|value| row.parsed_row().timing != Some(value))
            {
                return Ok(matched);
            }
            ensure!(
                matched.is_none(),
                "multiple person-owned links match the original mark"
            );
            Ok(Some(linked))
        })?
        .context("source-owned person has no exact linked meet/document/day/mark match")?;
    let held = snapshot
        .read_retained_capture_root_v1(provider.root().schema, &provider.root().key, grant)?
        .context("provider retained root absent")?;
    ensure!(held.root() == provider.root(), "provider root changed");
    grant.charge_bytes(held.payload().get().len())?;
    let packet: std::collections::BTreeMap<&str, &serde_json::value::RawValue> =
        serde_json::from_str(held.payload().get())?;
    let linked_raw = packet
        .get("linked_results")
        .context("provider linked capture absent")?;
    let capture: census_crawl::participation_evidence::RetainedHtmlCaptureV1 =
        serde_json::from_str(linked_raw.get())?;
    ensure!(
        capture.body_sha256() == linked.proof().capture(),
        "linked proof capture differs"
    );
    let capture_ref = objects.leaf(
        provider.root(),
        RetainedCaptureSlotV1::ProviderLinkedResults,
        RetainedCaptureLeafKindV1::CaptureJson,
        reference(linked_raw.get().as_bytes())?,
        grant,
    )?;
    let body_ref = objects.leaf(
        provider.root(),
        RetainedCaptureSlotV1::ProviderLinkedResults,
        RetainedCaptureLeafKindV1::OriginalBody,
        capture.original_body_reference()?,
        grant,
    )?;
    let fact = original_link(
        linked_raw,
        &capture,
        linked.proof().locator(),
        objects,
        grant,
    )?;
    let target = objects.json(
        &(
            provider.root(),
            profile.person(),
            profile.school(),
            profile.name(),
            profile.cohort(),
            capture_ref,
            body_ref,
            &fact,
            linked.event_code(),
            linked.proof().locator(),
            linked.proof().interpretation(),
        ),
        grant,
    )?;
    Ok((target, fact))
}

fn checked_event_context(
    snapshot: &StoreSnapshot<'_>,
    provider: &RecordedProviderTargetV1,
    document: &CheckedRetainedResultDocumentV1<'_>,
    row: &CheckedRetainedResultRowV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<(
    CheckedLinkedEventLabelV1,
    AttachmentDigestV1,
    AttachmentDigestV1,
)> {
    let linked = provider
        .checked()
        .categories()
        .rows()
        .iter()
        .try_fold(None, |matched, candidate| -> anyhow::Result<_> {
            grant.charge_steps(1)?;
            if candidate.event_code() != "56"
                || candidate.meet_id() != document.meet_id()
                || candidate.result_document_id() != document.result_document_id()
                || candidate.meet_day().as_str() != document.published_start()
                || candidate.category() != Some(row.event().gender)
            {
                return Ok(matched);
            }
            let (time, timing) =
                parse_published_time(candidate.mark()).context("provider linked mark")?;
            if row.parsed_row().mark != Mark::TimeSeconds(time)
                || timing.is_some_and(|value| row.parsed_row().timing != Some(value))
            {
                return Ok(matched);
            }
            ensure!(matched.is_none(), "multiple exact provider event rows");
            Ok(Some(candidate))
        })?
        .context("exact provider event row absent")?;
    let held = snapshot
        .read_retained_capture_root_v1(provider.root().schema, &provider.root().key, grant)?
        .context("provider retained root absent")?;
    ensure!(held.root() == provider.root(), "provider root changed");
    let packet: SourceTargetPacketV1 = serde_json::from_str(held.payload().get())?;
    ensure!(
        packet.reference()?.as_str() == provider.root().key,
        "native provider packet reference differs from retained root"
    );
    let label = packet.read_event_label(linked.proof())?;
    ensure!(
        label.person() == provider.checked().profile().person(),
        "checked event label person differs from provider target"
    );
    let fields: std::collections::BTreeMap<&str, &serde_json::value::RawValue> =
        serde_json::from_str(held.payload().get())?;
    let linked_raw = fields
        .get("linked_results")
        .context("provider linked capture absent")?;
    let capture: census_crawl::participation_evidence::RetainedHtmlCaptureV1 =
        serde_json::from_str(linked_raw.get())?;
    ensure!(
        capture.body_sha256() == linked.proof().capture(),
        "linked proof capture differs"
    );
    let capture_ref = objects.leaf(
        provider.root(),
        RetainedCaptureSlotV1::ProviderLinkedResults,
        RetainedCaptureLeafKindV1::CaptureJson,
        reference(linked_raw.get().as_bytes())?,
        grant,
    )?;
    let body_ref = objects.leaf(
        provider.root(),
        RetainedCaptureSlotV1::ProviderLinkedResults,
        RetainedCaptureLeafKindV1::OriginalBody,
        capture.original_body_reference()?,
        grant,
    )?;
    let heading = original_link(
        linked_raw,
        &capture,
        label.heading_proof().locator(),
        objects,
        grant,
    )?;
    ensure!(
        heading == label.heading_reference().digest,
        "checked heading bytes differ from retained heading"
    );
    let scope = objects.json(
        &(
            provider.root(),
            capture_ref,
            body_ref,
            label.source_url(),
            (
                "heading-proof/v1",
                label.heading_proof().capture(),
                label.heading_proof().interpretation(),
                label.heading_proof().locator(),
                label.heading_proof().acquired_at(),
            ),
            (
                "table-proof/v1",
                label.table_proof().capture(),
                label.table_proof().interpretation(),
                label.table_proof().locator(),
                label.table_proof().acquired_at(),
            ),
            (
                "row-proof/v1",
                label.row_proof().capture(),
                label.row_proof().interpretation(),
                label.row_proof().locator(),
                label.row_proof().acquired_at(),
            ),
            label.row_reference(),
            label.heading_reference(),
        ),
        grant,
    )?;
    Ok((label, heading, scope))
}

fn build_command(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    subject: RetainedDocumentSubjectReferenceV1,
    row_fact: AttachmentDigestV1,
    target: (AttachmentDigestV1, AttachmentDigestV1),
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<RetainedEvidenceSubjectAttachmentCommandV1> {
    let lineage = objects.json(&("reviewed-publisher-family/v1", "milesplit.com"), grant)?;
    let mut facts = Vec::new();
    reserve(&mut facts, 2, grant)?;
    facts.push(ReviewedAttachmentFactV1 { fact: row_fact.clone(), subject: subject.subject.clone(), publisher_lineage: lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Original captured result row; provider-owned linked meet, document, day and mark checked without accepted-fact injection")? });
    facts.push(ReviewedAttachmentFactV1 { fact: target.1.clone(), subject: target.0.clone(), publisher_lineage: lineage, disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Checked provider person and current source binding; exact published linked-result context, not an independent publisher")? });
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let pair = AttachmentCorroborationPairV1 { subject_fact: row_fact, accepted_target_fact: target.1, basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference, explanation: text("Same-publisher explicit person-owned result link names this original meet/document/day/mark; unauthenticated local operator provenance only")? };
    let scope = objects.json(
        &(
            &subject,
            &facts,
            &pair,
            &target.0,
            inspection.target(),
            inspection.accepted_target_witness(),
            inspection.public_head(),
            inspection.inventory(),
        ),
        grant,
    )?;
    let command = RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: inspection.store_identity().clone(), operation: text(OPERATION)?,
        expected_public_head: inspection.public_head().clone(), expected_evidence_generation: inspection.evidence_generation(), expected_derived_generation: inspection.derived_generation(), expected_attachment_inventory: inspection.inventory().clone(), target: inspection.target().clone(),
        subjects: list([subject], grant)?, action: inspection.action().clone(),
        reviewed_context: ReviewedAttachmentContextV1 { objects: objects.references(inspection, grant)?, facts: AttachmentListV1::new(facts)?, corroboration: list([pair], grant)?, complete_scope_records: list([scope], grant)?, accepted_target_subjects: list([target.0], grant)?, accepted_target_witness: inspection.accepted_target_witness().clone() },
        actor: AttachmentReviewActorV1::LocalOperator, reviewed_at: Utc::now(), rationale: text("Unauthenticated local operator approval of an untrusted source-derived proposal; no authenticated human review or independent-source claim")?,
    };
    let charged = encode(&command, grant)?;
    grant.charge_bytes(charged.len())?;
    Ok(command)
}

fn merged_objects(
    original: &[AttachmentObjectReferenceV1],
    additions: &[AttachmentObjectReferenceV1],
) -> anyhow::Result<AttachmentListV1<AttachmentObjectReferenceV1>> {
    let mut values = original.to_vec();
    values.extend_from_slice(additions);
    values.sort();
    ensure!(
        values.windows(2).all(|pair| {
            !matches!(pair, [left, right] if left.digest == right.digest && left != right)
        }),
        "context object digest has conflicting lengths"
    );
    values.dedup_by(|left, right| left.digest == right.digest);
    Ok(AttachmentListV1::new(values)?)
}

fn event_context_command(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    original: &ReviewedReferenceRecordV1,
    subject: &RetainedDocumentSubjectReferenceV1,
    row_fact: &AttachmentDigestV1,
    heading: AttachmentDigestV1,
    scope: AttachmentDigestV1,
    objects: &Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<RetainedEvidenceSubjectAttachmentCommandV1> {
    let original_command = &original.command;
    let original_ref = AttachmentRecordReferenceV1 {
        operation: original_command.operation.clone(),
        digest: original.record_digest.clone(),
    };
    ensure!(
        inspection.target() == &original_command.target
            && inspection.accepted_target_witness()
                == &original_command.reviewed_context.accepted_target_witness
            && matches!(
                inspection.action(),
                AttachmentActionV1::Attach {
                    prior: AttachmentPredecessorV1::After(prior)
                } if prior == &original_ref
            ),
        "event-context inspection is not after the original attachment"
    );
    let lineage = original_command
        .reviewed_context
        .facts
        .as_slice()
        .iter()
        .find(|fact| fact.fact == *row_fact)
        .context("original raw-row fact has no publisher lineage")?
        .publisher_lineage
        .clone();
    let mut facts = original_command.reviewed_context.facts.as_slice().to_vec();
    facts.push(ReviewedAttachmentFactV1 {
        fact: heading.clone(),
        subject: subject.subject.clone(),
        publisher_lineage: lineage,
        disposition: AttachmentFactDispositionV1::ContextOnly,
        rationale: text(
            "Exact retained provider h3 event heading and linked row context; context-only, not an independent publisher or human-authenticated claim",
        )?,
    });
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    ensure!(
        facts
            .windows(2)
            .all(|pair| matches!(pair, [first, second] if first.fact != second.fact)),
        "event-context fact digest duplicates original fact"
    );
    let mut scopes = original_command
        .reviewed_context
        .complete_scope_records
        .as_slice()
        .to_vec();
    scopes.push(scope);
    scopes.sort();
    ensure!(
        scopes
            .windows(2)
            .all(|pair| matches!(pair, [first, second] if first != second)),
        "event-context scope digest duplicates original scope"
    );
    let additions = objects.references(inspection, grant)?;
    let reviewed_context = ReviewedAttachmentContextV1 {
        objects: merged_objects(
            original_command.reviewed_context.objects.as_slice(),
            additions.as_slice(),
        )?,
        facts: AttachmentListV1::new(facts)?,
        corroboration: original_command.reviewed_context.corroboration.clone(),
        complete_scope_records: AttachmentListV1::new(scopes)?,
        accepted_target_subjects: original_command
            .reviewed_context
            .accepted_target_subjects
            .clone(),
        accepted_target_witness: inspection.accepted_target_witness().clone(),
    };
    let command = RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1,
        policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: text(EVENT_CONTEXT_OPERATION)?,
        expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(),
        expected_derived_generation: inspection.derived_generation(),
        expected_attachment_inventory: inspection.inventory().clone(),
        target: inspection.target().clone(),
        subjects: original_command.subjects.clone(),
        action: inspection.action().clone(),
        reviewed_context,
        actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: Utc::now(),
        rationale: text(
            "Unauthenticated local operator review of exact retained event heading context; no independent publisher or authenticated human claim",
        )?,
    };
    let charged = encode(&command, grant)?;
    grant.charge_bytes(charged.len())?;
    Ok(command)
}

fn retain(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
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
            ensure!(
                std::fs::symlink_metadata(path)?.file_type().is_file(),
                "artifact is not a regular file: {}",
                path.display()
            );
            let mut file = std::fs::File::open(path)?;
            ensure!(
                file.metadata()?.len() == u64::try_from(bytes.len())?,
                "refusing to replace original artifact: {}",
                path.display()
            );
            let mut buffer = [0u8; 4096];
            bytes
                .chunks(4096)
                .try_for_each(|chunk| -> anyhow::Result<()> {
                    let held = buffer
                        .get_mut(..chunk.len())
                        .context("artifact comparison buffer")?;
                    file.read_exact(held)?;
                    ensure!(
                        held == chunk,
                        "refusing to replace original artifact: {}",
                        path.display()
                    );
                    Ok(())
                })?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn read_bounded(
    path: &Path,
    limit: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let length = file.metadata()?.len();
    ensure!(
        length <= limit,
        "retained artifact exceeds bounded read limit: {}",
        path.display()
    );
    let size = usize::try_from(length)?;
    let mut bytes = Vec::new();
    reserve(&mut bytes, size, grant)?;
    bytes.resize(size, 0);
    file.read_exact(&mut bytes)?;
    ensure!(
        file.read(&mut [0])? == 0,
        "retained artifact grew during read"
    );
    Ok(bytes)
}

fn apply_named(
    store: &Store,
    prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
    artifacts: &Path,
    prefix: &str,
    require_repeated: bool,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(artifacts)?;
    let approval = if require_repeated && prepared.is_replay() {
        let retained = read_bounded(
            &artifacts.join(format!("{prefix}-approval.bin")),
            1024 * 1024,
            grant,
        )?;
        ensure!(
            retained == prepared.approval_bytes(),
            "retained approval differs from ReplayOnly preparation"
        );
        retained
    } else {
        let mut retained = Vec::new();
        reserve(&mut retained, prepared.approval_bytes().len(), grant)?;
        retained.extend_from_slice(prepared.approval_bytes());
        retained
    };
    let digest = if require_repeated && prepared.is_replay() {
        let retained = read_bounded(
            &artifacts.join(format!("{prefix}-approval.digest")),
            256,
            grant,
        )?;
        let parsed = AttachmentDigestV1::parse(std::str::from_utf8(&retained)?)?;
        ensure!(
            parsed == *prepared.approval_digest(),
            "retained approval digest differs from ReplayOnly preparation"
        );
        parsed
    } else {
        prepared.approval_digest().clone()
    };
    ensure!(
        prepared.command().approval_digest_with_grant(grant)? == digest,
        "approval digest differs from original command"
    );
    retain(&artifacts.join(format!("{prefix}-approval.bin")), &approval)?;
    retain(
        &artifacts.join(format!("{prefix}-approval.digest")),
        digest.as_str().as_bytes(),
    )?;
    retain(
        &artifacts.join(format!("{prefix}-approval.sha256")),
        reference(&approval)?.digest.as_str().as_bytes(),
    )?;
    retain(
        &artifacts.join(format!("{prefix}-command.json")),
        &encode(prepared.command(), grant)?,
    )?;
    let result = store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?;
    let receipt = match result {
        Application::Repeated(receipt) => receipt,
        Application::Written(receipt) if !require_repeated => receipt,
        Application::Written(_) => anyhow::bail!("ReplayOnly application wrote a new record"),
    };
    if require_repeated {
        let retained = read_bounded(
            &artifacts.join(format!("{prefix}-receipt.json")),
            64 * 1024,
            grant,
        )?;
        let original: census_store::Receipt = serde_json::from_slice(&retained)?;
        ensure!(
            original == receipt,
            "replay receipt differs from retained original receipt"
        );
    }
    retain(
        &artifacts.join(format!("{prefix}-receipt.json")),
        &encode(&receipt, grant)?,
    )?;
    ensure!(
        store.receipt(&receipt.operation)?.as_ref() == Some(&receipt),
        "committed attachment receipt differs"
    );
    Ok(())
}

fn apply(
    store: &Store,
    prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
    artifacts: &Path,
    grant: &mut AttachmentReplayGrantV1,
) -> anyhow::Result<()> {
    apply_named(store, prepared, artifacts, "attachment", false, grant)
}

fn owned_text(value: &str, grant: &mut AttachmentReplayGrantV1) -> anyhow::Result<String> {
    grant.charge_bytes(value.len())?;
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
    result.push_str(value);
    Ok(result)
}

pub(super) fn ensure_person(
    store: &Store,
    candidate: &AthleteId,
    context: &str,
    target_ref: &str,
    locator: SourceRowLocator,
    artifacts: &Path,
) -> anyhow::Result<(String, String)> {
    let mut grant = AttachmentReplayGrantV1::new();
    ensure!(
        candidate.validate("ath_subject"),
        "invalid source-bound athlete identifier"
    );
    grant.charge_bytes(candidate.as_str().len())?;
    let budgeted: AthleteCandidateId = candidate.cast();
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .context("accepted public identity authority absent")?;
    let public = match accepted.resolve_candidate(&budgeted)? {
        PublicResolution::Canonical(value) => value.public_id,
        other => anyhow::bail!("candidate has no canonical public resolution: {other:?}"),
    };
    let retained = read_recorded_html_result_capture(&snapshot, context, &[locator], &mut grant)?
        .context("retained result capture absent")?;
    grant.charge_bytes(usize::try_from(
        retained.capture().original_body_reference()?.bytes,
    )?)?;
    let document = retained.read(&[locator])?;
    ensure!(
        document.selected_count() == 1,
        "document must select exactly one original row"
    );
    let row: CheckedRetainedResultRowV1<'_> = document.row(0).context("original row absent")?;
    ensure!(
        row.locator() == locator && document.context_reference() == context,
        "original locator or context differs"
    );
    let mut objects = Objects::default();
    let (subject, fact) = row_subject(&document, retained.root(), &mut objects, &mut grant)?;
    let provider = provider_target(&snapshot, &accepted, &budgeted, target_ref, &mut grant)?;
    let target = target_context(&snapshot, &provider, &document, &mut objects, &mut grant)?;
    let records = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    if let Some(record) = records
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == OPERATION)
    {
        ensure!(
            record.command.target.public_id == public
                && record.command.target.canonical_candidate == budgeted
                && record.command.subjects.as_slice() == std::slice::from_ref(&subject),
            "original operation belongs to another target or source row"
        );
        ensure!(
            record
                .command
                .reviewed_context
                .accepted_target_subjects
                .as_slice()
                == std::slice::from_ref(&target.0)
                && record
                    .command
                    .reviewed_context
                    .facts
                    .as_slice()
                    .iter()
                    .any(|fact| fact.subject == target.0 && fact.fact == target.1),
            "original operation belongs to another retained provider packet or linked row"
        );
        let bytes = encode(&record.command, &mut grant)?;
        grant.charge_bytes(bytes.len())?;
        let original = serde_json::from_slice(&bytes)?;
        let prepared = store.prepare_evidence_subject_attachment_v1(
            original,
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        )?;
        ensure!(
            prepared.is_replay(),
            "retained command did not prepare as exact replay"
        );
        apply_named(store, prepared, artifacts, "attachment", true, &mut grant)?;
        return Ok((
            owned_text(record.record_digest.as_str(), &mut grant)?,
            owned_text(public.as_str(), &mut grant)?,
        ));
    } else {
        let inspection = store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: public.clone(),
                prior: AttachmentPredecessorV1::First,
            },
        )?;
        ensure!(inspection.target().canonical_candidate == budgeted && inspection.public_head().sequence == accepted.head().sequence && inspection.public_head().digest.as_str() == accepted.head().digest && inspection.inventory() == records.inventory() && inspection.evidence_generation() == snapshot.evidence_generation() && inspection.derived_generation() == snapshot.derived_generation(), "inspection no longer matches original accepted view, generations or attachment inventory");
        let command = build_command(
            &inspection,
            subject.clone(),
            fact,
            target,
            &mut objects,
            &mut grant,
        )?;
        let borrowed = objects.borrowed(&mut grant)?;
        let prepared = store.prepare_evidence_subject_attachment_v1(
            command,
            EvidenceSubjectAttachmentPreparationV1::New {
                inspection,
                context: &borrowed,
            },
        )?;
        apply(store, prepared, artifacts, &mut grant)?;
    }
    let current_snapshot = store.snapshot();
    let current =
        current_snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let active = current
        .bind_exact_subject(&subject, &mut grant)?
        .context("original result row attachment is not currently visible")?;
    active.require_original_grant(&mut grant)?;
    ensure!(
        active.target().public_id == public
            && active.target().canonical_candidate == budgeted
            && active.record().command.operation.as_str() == OPERATION,
        "visible original row attachment or public identity differs"
    );
    retain(
        &artifacts.join("attachment-record.json"),
        &encode(active.record(), &mut grant)?,
    )?;
    Ok((
        owned_text(active.record().record_digest.as_str(), &mut grant)?,
        owned_text(public.as_str(), &mut grant)?,
    ))
}
pub(super) fn ensure_event_context(
    store: &Store,
    candidate: &AthleteId,
    context: &str,
    target_ref: &str,
    locator: SourceRowLocator,
    artifacts: &Path,
) -> anyhow::Result<String> {
    let mut grant = AttachmentReplayGrantV1::new();
    ensure!(
        candidate.validate("ath_subject"),
        "invalid source-bound athlete identifier"
    );
    grant.charge_bytes(candidate.as_str().len())?;
    let budgeted: AthleteCandidateId = candidate.cast();
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .context("accepted public identity authority absent")?;
    let public = match accepted.resolve_candidate(&budgeted)? {
        PublicResolution::Canonical(value) => value.public_id,
        other => anyhow::bail!("candidate has no canonical public resolution: {other:?}"),
    };
    let retained = read_recorded_html_result_capture(&snapshot, context, &[locator], &mut grant)?
        .context("retained result capture absent")?;
    grant.charge_bytes(usize::try_from(
        retained.capture().original_body_reference()?.bytes,
    )?)?;
    let document = retained.read(&[locator])?;
    ensure!(
        document.selected_count() == 1,
        "document must select one original row"
    );
    let row = document.row(0).context("original row absent")?;
    ensure!(
        row.locator() == locator && document.context_reference() == context,
        "original locator or context differs"
    );
    let mut objects = Objects::default();
    let (subject, row_fact) = row_subject(&document, retained.root(), &mut objects, &mut grant)?;
    let provider = provider_target(&snapshot, &accepted, &budgeted, target_ref, &mut grant)?;
    let _target = target_context(&snapshot, &provider, &document, &mut objects, &mut grant)?;
    let (_label, heading, scope) = checked_event_context(
        &snapshot,
        &provider,
        &document,
        &row,
        &mut objects,
        &mut grant,
    )?;
    let records = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    let original = records
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == OPERATION)
        .context("original result attachment absent")?;
    ensure!(
        original.command.target.public_id == public
            && original.command.target.canonical_candidate == budgeted
            && original.command.subjects.as_slice() == std::slice::from_ref(&subject),
        "original result attachment belongs to another target or source row"
    );
    let original_ref = AttachmentRecordReferenceV1 {
        operation: original.command.operation.clone(),
        digest: original.record_digest.clone(),
    };
    let existing = records
        .records()
        .iter()
        .find(|record| record.command.operation.as_str() == EVENT_CONTEXT_OPERATION);
    if let Some(record) = existing {
        ensure!(
            record.command.target == original.command.target
                && record.command.subjects == original.command.subjects
                && matches!(
                    &record.command.action,
                    AttachmentActionV1::Attach {
                        prior: AttachmentPredecessorV1::After(prior)
                    } if prior == &original_ref
                ),
            "existing event-context operation belongs to another source"
        );
        let prepared = store.prepare_evidence_subject_attachment_v1(
            record.command.clone(),
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        )?;
        ensure!(prepared.is_replay(), "event-context replay was not exact");
        apply_named(
            store,
            prepared,
            artifacts,
            "distinct-event-context",
            true,
            &mut grant,
        )?;
    } else {
        let inspection = store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: public.clone(),
                prior: AttachmentPredecessorV1::After(original_ref),
            },
        )?;
        ensure!(
            inspection.target().canonical_candidate == budgeted
                && inspection.public_head().sequence == accepted.head().sequence
                && inspection.public_head().digest.as_str() == accepted.head().digest
                && inspection.inventory() == records.inventory()
                && inspection.evidence_generation() == snapshot.evidence_generation()
                && inspection.derived_generation() == snapshot.derived_generation(),
            "event-context inspection no longer matches accepted view"
        );
        let command = event_context_command(
            &inspection,
            original,
            &subject,
            &row_fact,
            heading,
            scope,
            &objects,
            &mut grant,
        )?;
        let borrowed = objects.borrowed_excluding(
            original.command.reviewed_context.objects.as_slice(),
            &mut grant,
        )?;
        let prepared = store.prepare_evidence_subject_attachment_v1(
            command,
            EvidenceSubjectAttachmentPreparationV1::New {
                inspection,
                context: &borrowed,
            },
        )?;
        apply_named(
            store,
            prepared,
            artifacts,
            "distinct-event-context",
            false,
            &mut grant,
        )?;
    }
    let current_snapshot = store.snapshot();
    let current =
        current_snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let active = current
        .bind_exact_subject(&subject, &mut grant)?
        .context("event-context attachment is not currently visible")?;
    active.require_original_grant(&mut grant)?;
    ensure!(
        active.target().public_id == public
            && active.target().canonical_candidate == budgeted
            && active.record().command.operation.as_str() == EVENT_CONTEXT_OPERATION,
        "visible event-context attachment differs"
    );
    retain(
        &artifacts.join("distinct-event-context-record.json"),
        &encode(active.record(), &mut grant)?,
    )?;
    owned_text(active.record().record_digest.as_str(), &mut grant)
}
