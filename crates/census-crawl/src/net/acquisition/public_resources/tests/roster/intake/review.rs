use super::*;
use crate::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, assess_retained_roster_source_v3,
    assess_reviewed_roster_source_v1, manifest::compiled_roster_parser_manifest,
    policies::compiled_roster_interpretation_contracts_v1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalSchool, CanonicalTeam, Gender, SchoolYear, SourceIdentity, SourceNamespace,
    SourceObservation, Sport,
};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, FrozenRosterReviewedClaimsBundleV1,
    PreparedRosterSourceReviewV1, ReviewedRosterSourceClaimV1, RosterEnrollmentReviewV1,
    RosterIssuanceInspectionRequestV1, RosterReviewActorV1, RosterSourceReviewDecisionV1,
    RosterSourceReviewFieldsV1, Table,
};

mod proposal;
mod recorded_school;

const OPERATION: &str = "controlled-owned-genuine-school-review";

fn review_fixture(listener: &TcpListener) -> TestResult<Fixture> {
    let fixture = Fixture::new(listener)?;
    let roots = [
        (
            PublicResourceRootKindV1::Robots,
            fixture.work(PublicResourceRootKindV1::Robots)?,
        ),
        (
            PublicResourceRootKindV1::MileSplitTeamDirectory,
            fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?,
        ),
    ];
    let identity = fixture.writer.store_identity().clone();
    let Fixture {
        root,
        store,
        writer,
        owner,
        fetcher,
    } = fixture;
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?,
        census_store::PublicIdentityInputRevisionsV2::Disabled);
    store.flush()?;
    drop((owner, writer, store));
    let location = root.path().join("store");
    let (store, identity_writer) =
        Store::open_public_identity_qualification_v3(&location)?.into_parts();
    let token = identity_writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?,
        census_store::PublicIdentityInputRevisionsV2::Enabled(token));
    check!(eq; identity_writer.store_identity(), &identity);
    store.flush()?;
    drop((identity_writer, store));
    let (store, writer) = Store::open_current_qualification_v3(&location)?.into_parts();
    check!(eq; writer.store_identity(), &identity);
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?,
        census_store::PublicIdentityInputRevisionsV2::Enabled(token));
    let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
    for (kind, held) in roots {
        let replayed = owner.for_root(&store, kind)?;
        check!(eq; replayed.id()?, held.id()?);
        check!(eq; replayed, held);
    }
    Ok(Fixture {
        root,
        store,
        writer,
        owner,
        fetcher,
    })
}

fn enroll(store: &Store) -> TestResult {
    let prepared = store.prepare_roster_source_enrollment_v1(
        store.inspect_roster_source_enrollment_v1()?,
        RosterEnrollmentReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-owned-roster-enrollment")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: chrono::Utc::now(),
            rationale: RosterAuthorityTextV1::parse("Controlled source review test enrollment")?,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_enrollment_v1(prepared, &approved, &digest)?
        .written());
    Ok(())
}

fn parents(
    store: &Store,
    packet: &PublicSchoolIntakePacketV1,
) -> TestResult<(CanonicalSchool, CanonicalTeam)> {
    let checked = OwnedProviderSchoolPacketV1::from_packet(packet.clone()).read()?;
    verify_school(&checked)?;
    let school = checked.school().clone();
    let day = packet.roster().response().observed_at();
    let instant = chrono::DateTime::parse_from_rfc3339(day)?.to_utc();
    let year =
        SchoolYear::from_date(&instant.date_naive().to_string()).ok_or("capture school year")?;
    let program = CanonicalTeam {
        id: CanonicalTeam::mint(&school.id, Sport::IndoorTrack, Gender::Boys, year),
        school: school.id.clone(),
        sport: Sport::IndoorTrack,
        gender: Gender::Boys,
        school_year: year,
        level: None,
        source_identities: vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, TEAM)],
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Schools, &school)?;
    store.append(Table::Teams, &program)?;
    store.append(
        Table::SourceObservations,
        &SourceObservation::School(checked.observation().clone()),
    )?;
    Ok((school, program))
}

fn prepare_review<'a>(
    store: &'a Store,
    held: &Held,
) -> TestResult<PreparedRosterSourceReviewV1<'a>> {
    let (school, program) = parents(store, &held.packet)?;
    let mut inspection = store.inspect_roster_source_issuance_v1(
        RosterIssuanceInspectionRequestV1 {
            root: held.root.clone(),
            school: school.id.clone(),
            program: program.id.clone(),
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let (claim, objects) = proposal::derive(&held.packet, &school, &program, &mut inspection)?;
    Ok(store.prepare_roster_source_review_v1(
        inspection,
        claim,
        RosterSourceReviewFieldsV1 {
            operation: RosterAuthorityTextV1::parse(OPERATION)?,
            actor: RosterReviewActorV1::LocalOperator,
            decision: RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
            reviewed_at: chrono::Utc::now(),
            rationale: RosterAuthorityTextV1::parse(
                "Controlled exact source review; no identity or enrollment authority",
            )?,
        },
        objects,
    )?)
}

fn issue(store: &Store, held: &Held) -> TestResult<(Receipt, String)> {
    let mut prepared = prepare_review(store, held)?;
    let claim = prepared.proposal_claim()?;
    let compiled = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let (evidence, grant) = prepared.source_parts()?;
    let source = assess_retained_roster_source_v3(
        evidence,
        compiled.canonical_bytes(),
        &contracts,
        &claim,
        grant,
    )?;
    assess_existing_roster_binding(&source, grant)?;
    check!(eq; source.scoped_rows(grant)?, 33);
    check!(eq; source.retained_packet(grant)?.encode()?, held.bytes);
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_review_v1(prepared, &approval, &digest)?;
    check!(applied.written());
    let repeated = store.replay_roster_source_review_v1(OPERATION, &digest)?;
    check!(repeated.repeated());
    check!(eq; repeated.receipt(), applied.receipt());
    Ok((applied.receipt().clone(), digest))
}

fn reassess(
    claim: &ReviewedRosterSourceClaimV1,
    held: &Held,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let reviewed = assess_reviewed_roster_source_v1(claim, grant)?;
    check!(eq; reviewed.school(grant)?.name.as_str(), "Detroit Catholic Central");
    let source = reviewed.source(grant)?;
    check!(eq; source.root(grant)?, &held.root);
    check!(eq; source.retained_packet(grant)?.encode()?, held.bytes);
    check!(eq; source.roster_body(grant)?.as_bytes(), ROSTER);
    check!(eq; source.scoped_rows(grant)?, 33);
    verify_school(source.checked_packet(grant)?)?;
    Ok(())
}

fn freeze(store: &Store, held: &Held) -> TestResult<FrozenRosterReviewedClaimsBundleV1> {
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claims = current.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    reassess(
        claims.first().ok_or("current observed source claim")?,
        held,
        &mut grant,
    )?;
    Ok(current.freeze_closure(&mut grant)?)
}

#[tokio::test]
async fn minted_genuine_observed_packet_survives_review_current_cold_and_store_free_frozen_read(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = review_fixture(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let held = mint(&fixture, &directory, &roster)?;
    enroll(&fixture.store)?;
    let (receipt, digest) = issue(&fixture.store, &held)?;
    let original = freeze(&fixture.store, &held)?;
    let revisions = fixture
        .store
        .snapshot()
        .public_identity_input_revisions_v2()?;
    check!(matches!(
        revisions,
        census_store::PublicIdentityInputRevisionsV2::Enabled(_)
    ));
    let fixture = fixture.reopen()?;
    check!(eq; fixture.store.snapshot().public_identity_input_revisions_v2()?, revisions);
    let repeated = fixture
        .store
        .replay_roster_source_review_v1(OPERATION, &digest)?;
    check!(eq; repeated.receipt(), &receipt);
    let cold = freeze(&fixture.store, &held)?;
    check!(eq; cold.bytes(), original.bytes());
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    let Fixture {
        root,
        store,
        writer,
        owner,
        fetcher,
    } = fixture;
    store.flush()?;
    drop((owner, writer, store, fetcher));
    root.close()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed = replay_frozen_roster_reviewed_claims_v1(
        original.bytes(),
        original.reference(),
        &mut grant,
    )?;
    let claims = replayed.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    reassess(
        claims.first().ok_or("frozen observed source claim")?,
        &held,
        &mut grant,
    )?;
    Ok(())
}
