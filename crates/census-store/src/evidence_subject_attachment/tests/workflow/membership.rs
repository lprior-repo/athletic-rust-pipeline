use super::fixture::*;
use crate::evidence_subject_attachment::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{
    IdentityCommitOutcomeV1, PublicIdentityWriterV1, ReviewedIdentityRequestV1, Store, StoreError,
    Table,
};
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{
    AthleteIdentityIndex, AttestationQualification, CanonicalAthlete, Evidence,
    IdentityAttestation, ReviewCase, ReviewState, ReviewVerdictRecord, SourceIdentity,
    SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::public_identity_history::PublicResolution;

struct MergeFixture {
    _directory: tempfile::TempDir,
    store: Store,
    writer: PublicIdentityWriterV1,
    births: [IdentityCommitOutcomeV1; 2],
    request: ReviewedIdentityRequestV1,
}

fn ready_for_merge() -> TestResult<MergeFixture> {
    let (directory, store, writer) = fixture()?;
    let mut first = subject("attachment-reviewed-merge", "93201")?;
    let source = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "93202");
    let mut second = CanonicalAthlete::new(
        &first.school,
        "Synthetic Runner attachment-reviewed-merge-second",
        first.grad_year,
        first.gender,
        source.clone(),
    )?;
    second.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic timer fixture",
            Some("https://synthetic-timer.test/athletes/93202".into()),
        ),
        "2026-10-14",
    ));
    check!(ne; first.candidate_key().index_id(), second.candidate_key().index_id());
    let mut isolated = AthleteIdentityIndex::default();
    isolated.observe(&first)?;
    isolated.observe(&second)?;
    check!(isolated.isolated_source(&first.id.cast()));
    check!(isolated.isolated_source(&second.id.cast()));
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let first_birth = writer.establish_source_bound(
        &store,
        "attachment-merge-first-birth",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let second_birth = writer.establish_source_bound(
        &store,
        "attachment-merge-second-birth",
        &second.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    first.add_identity(source.clone());
    first.identity_attestations.push(claim(
        &source,
        "synthetic-school",
        "synthetic-school-publisher",
    )?);
    second.identity_attestations.push(claim(
        &source,
        "synthetic-timer",
        "synthetic-timer-publisher",
    )?);
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let mut index = AthleteIdentityIndex::default();
    index.observe(&first)?;
    index.observe(&second)?;
    let members = vec![first.id.cast(), second.id.cast()];
    let label = "Synthetic attachment membership fixture";
    let detail = "Controlled paired-source review for public identity merge";
    let evidence = index.case_evidence(label, detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        label,
        detail,
        evidence,
    );
    case.member_ids = members;
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: detail.into(),
        reviewer: "synthetic deterministic test review; no genuine person acceptance".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    };
    store.replace_many(Table::ReviewCases, &[case.clone()])?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let mut public_ids = vec![
        first_birth.public_id().clone(),
        second_birth.public_id().clone(),
    ];
    public_ids.sort();
    let request = ReviewedIdentityRequestV1::Merge {
        case_id: case.id,
        canonical: first_birth.public_id().clone(),
        members: public_ids,
    };
    Ok(MergeFixture {
        _directory: directory,
        store,
        writer,
        births: [first_birth, second_birth],
        request,
    })
}

fn claim(
    subject: &SourceIdentity,
    publisher: &str,
    producer: &str,
) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "synthetic_fixture": true, "published_subject": subject,
        "publisher": publisher, "producer": producer, "locator": "published_subject",
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: reference(&bytes)?.digest.as_str().to_owned(),
        acquired_at: "2026-10-14T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: producer.into(),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}

#[test]
fn actual_public_merge_changes_membership_and_refuses_old_active_target_replacement() -> TestResult
{
    let fixture = ready_for_merge()?;
    let first = fixture.births.first().ok_or("first birth")?;
    let context = Context::synthetic("membership-change")?;
    let inspection = inspect_first(&fixture.store, first.public_id())?;
    let command = context.command("membership-change", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let record = first_record(&fixture.store)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "attachment-membership-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: first.public_id().clone(),
                prior: AttachmentPredecessorV1::After(record.reference()),
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::TargetMoved
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; first_record(&fixture.store)?, record);
    Ok(())
}

#[test]
fn actual_public_redirect_does_not_prevent_revoking_the_exact_historical_attachment() -> TestResult
{
    let fixture = ready_for_merge()?;
    let second = fixture.births.get(1).ok_or("second birth")?;
    let context = Context::synthetic("redirected-revoke")?;
    let inspection = inspect_first(&fixture.store, second.public_id())?;
    let command = context.command("redirected-attachment", &inspection)?;
    let original_receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command.clone(), &context)?,
    )?)?;
    let original = first_record(&fixture.store)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "attachment-redirect-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted snapshot absent")?;
    let PublicResolution::Redirect { successor, .. } =
        accepted.resolve_public(second.public_id())?
    else {
        return Err("actual reviewed merge must redirect the second birth".into());
    };
    check!(eq; &successor.public_id, fixture.births.first().ok_or("first birth")?.public_id());
    let inspection = fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Revoke {
            prior: original.reference(),
        },
    )?;
    check!(eq; inspection.target(), &original.command.target);
    check!(eq; inspection.accepted_target_witness(), &original.command.reviewed_context.accepted_target_witness);
    let revoke = next_command(&original, "redirected-revocation", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        revoke,
        Preparation::New {
            inspection,
            context: &[],
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    let before_replay = retained_state(&fixture.store)?;
    check!(eq; replay(&fixture.store, command)?, crate::Application::Repeated(original_receipt));
    check!(eq; retained_state(&fixture.store)?, before_replay);
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), 2);
    check!(eq; read.records().first(), Some(&original));
    check!(eq; active_operations(&read)?, Vec::<String>::new());
    Ok(())
}
