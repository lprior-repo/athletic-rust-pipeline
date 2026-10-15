use super::super::VerifiedPublicIdentityView;
use crate::export::{store_lineage, DatasetLineage};
use census_domain::model::{
    AthleteIdentityProjection, AttestationQualification, CanonicalAthlete, Evidence, Gender,
    GradYear, IdentityAttestation, ReviewCase, ReviewState, ReviewVerdictRecord, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::public_identity_history::RegistryHead;
use census_store::{
    IdentityCommitOutcomeV1, PublicIdentityWriterV1, ReviewedIdentityCommitOutcomeV1,
    ReviewedIdentityRequestV1, Store, StoreSnapshot, Table,
};
use sha2::{Digest, Sha256};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) struct Fixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub writer: PublicIdentityWriterV1,
    pub athletes: [CanonicalAthlete; 3],
    pub births: [IdentityCommitOutcomeV1; 2],
    pub initial_head: RegistryHead,
    pub initial_lineage: DatasetLineage,
}

pub(crate) fn source_births() -> TestResult<Fixture> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("identity");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let first = athlete("first", SourceNamespace::MilesplitAthlete, "9401")?;
    let second = athlete("second", SourceNamespace::TfrrsAthlete, "9402")?;
    let unknown = athlete("unestablished", SourceNamespace::MilesplitAthlete, "9403")?;
    let athletes = [first, second, unknown];
    store.append_many(Table::Athletes, &athletes)?;
    let initial_head = writer.head(&store)?;
    let initial_lineage = store_lineage(&store)?;
    let [first, second, _] = &athletes;
    let first_birth = writer.establish_source_bound(
        &store,
        "report-first-birth",
        &first.id.cast(),
        store.snapshot().revision(),
        &initial_head,
    )?;
    let second_birth = writer.establish_source_bound(
        &store,
        "report-second-birth",
        &second.id.cast(),
        store.snapshot().revision(),
        first_birth.head(),
    )?;
    Ok(Fixture {
        directory,
        store,
        writer,
        athletes,
        births: [first_birth, second_birth],
        initial_head,
        initial_lineage,
    })
}

fn athlete(
    label: &str,
    namespace: SourceNamespace,
    source_id: &str,
) -> TestResult<CanonicalAthlete> {
    let school = SchoolId::mint("sch", &["synthetic-public-report", label]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        format!("Synthetic Runner {label}"),
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(namespace, source_id),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic report fixture",
            Some(format!(
                "https://synthetic-report.test/athletes/{source_id}"
            )),
        ),
        "2026-10-14",
    ));
    Ok(athlete)
}

pub(crate) fn projection(snapshot: &StoreSnapshot<'_>) -> TestResult<AthleteIdentityProjection> {
    Ok(census_store::build_athlete_identity_projection(
        snapshot.athlete_identity_index()?,
        &snapshot.scan(Table::ReviewCases)?,
        &snapshot.scan(Table::IdentityVerdicts)?,
        &snapshot.scan(Table::AthleteIdentityDecisions)?,
    )?)
}

pub(crate) fn capture(store: &Store) -> TestResult<(VerifiedPublicIdentityView, DatasetLineage)> {
    let snapshot = store.snapshot();
    let lineage = store_lineage(store)?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted public ledger")?;
    let head = accepted.head().clone();
    let view = VerifiedPublicIdentityView::capture(
        accepted,
        &lineage,
        &head,
        &snapshot.athletes()?,
        &projection(&snapshot)?,
    )?;
    Ok((view, lineage))
}

pub(crate) fn merge(fixture: &Fixture) -> TestResult<ReviewedIdentityCommitOutcomeV1> {
    let request = prepare_merge(fixture)?;
    let [_, second_birth] = &fixture.births;
    Ok(fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "report-reviewed-merge",
        &request,
        fixture.store.snapshot().revision(),
        second_birth.head(),
    )?)
}

pub(crate) fn prepare_merge(fixture: &Fixture) -> TestResult<ReviewedIdentityRequestV1> {
    let [first, second, _] = &fixture.athletes;
    let mut first = first.clone();
    let mut second = second.clone();
    let source = second.source.as_ref().ok_or("second source")?.clone();
    first.add_identity(source.clone());
    first.identity_attestations.push(capture_claim(
        &source,
        "synthetic-school-roster",
        "synthetic-school",
    )?);
    second.identity_attestations.push(capture_claim(
        &source,
        "synthetic-timer-result",
        "synthetic-timer",
    )?);
    fixture
        .store
        .append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let members = vec![first.id.cast(), second.id.cast()];
    let index = fixture.store.snapshot().athlete_identity_index()?;
    let subject = "Synthetic independently published report identity";
    let detail = "Two retained independent synthetic documents identify the same provider subject";
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        detail,
        index.case_evidence(subject, detail, &members)?,
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
        reviewer: "synthetic report fixture".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    };
    let mut cases: Vec<ReviewCase> = fixture.store.snapshot().scan(Table::ReviewCases)?;
    let mut verdicts: Vec<ReviewVerdictRecord> =
        fixture.store.snapshot().scan(Table::IdentityVerdicts)?;
    cases.push(case.clone());
    verdicts.push(verdict);
    fixture.store.replace_many(Table::ReviewCases, &cases)?;
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &verdicts)?;
    let [first_birth, second_birth] = &fixture.births;
    let mut members = vec![
        first_birth.public_id().clone(),
        second_birth.public_id().clone(),
    ];
    members.sort();
    Ok(ReviewedIdentityRequestV1::Merge {
        case_id: case.id,
        canonical: first_birth.public_id().clone(),
        members,
    })
}

fn capture_claim(
    subject: &SourceIdentity,
    publisher: &str,
    producer: &str,
) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "synthetic_fixture": true, "publisher": publisher, "producer": producer,
        "published_subject": subject, "locator": "published_subject",
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(&bytes)),
        acquired_at: "2026-10-14T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: producer.into(),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}
