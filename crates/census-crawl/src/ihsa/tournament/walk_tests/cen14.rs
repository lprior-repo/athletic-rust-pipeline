use super::*;
use census_domain::model::{
    AthleteIdentityIndex, AttestationQualification, IdentityAttestation, SourceIdentity,
};

#[test]
fn cen14_ihsa_captured_parse_retains_exact_subject_capture_after_reopen_without_qualifying_a_mirror(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new()?;
            let report = harness.run(&options()).await?;
            check!(eq; report.errors, 0, "notes: {:?}", report.notes);
            let rows = harness.scan::<CanonicalAthlete>(Table::Athletes)?;
            let row = finisher(&rows)?;
            let claim = captured_subject(row)?;
            mirror_withholds(row, claim)?;
            let encoded = serde_json::to_vec(&row.identity_attestations)?;
            let path = harness.dir.path().join("store");
            drop(harness.store);
            let reopened = Store::open(path)?;
            let retained = reopened.scan::<CanonicalAthlete>(Table::Athletes)?;
            check!(eq; serde_json::to_vec(&finisher(&retained)?.identity_attestations)?, encoded);
            Ok(())
        })
}

fn finisher(rows: &[CanonicalAthlete]) -> TestResult<&CanonicalAthlete> {
    rows.iter()
        .find(|row| row.canonical_name == "Kehlin Crawford")
        .ok_or_else(|| "real captured finisher".into())
}

fn captured_subject(row: &CanonicalAthlete) -> TestResult<&IdentityAttestation> {
    let claim = row
        .identity_attestations
        .iter()
        .find(|claim| {
            claim.subject.id == "27740691"
                && matches!(&claim.subject.namespace,
            SourceNamespace::AthleticNet { kind } if kind == "athlete")
        })
        .ok_or("exact captured subject claim")?;
    check!(eq; claim.capture_sha256, "ffa73baef90e494bb9b74b5473f98efd26ce083cf22c7efe12d80cf57eae0611");
    check!(eq; claim.acquired_at, "2026-09-20T14:39:00Z");
    check!(eq; claim.source.url.as_deref(), Some("https://api.ihsa.org/v1/track-field/events/2790204/summary"));
    check!(eq; claim.subject_locator, "https://api.ihsa.org/v1/track-field/events/2790204/summary:row:0");
    check!(eq; claim.qualification, AttestationQualification::CandidateOnly);
    Ok(claim)
}

fn mirror_withholds(row: &CanonicalAthlete, claim: &IdentityAttestation) -> TestResult {
    let subject = row.source.as_ref().ok_or("captured primary subject")?;
    check!(eq; subject.id, "27740691");
    let mut linked = CanonicalAthlete::new(
        &row.school,
        &row.canonical_name,
        row.grad_year,
        row.gender,
        SourceIdentity::new(
            SourceNamespace::Other("ihsa-row".into()),
            &claim.subject_locator,
        ),
    );
    linked.add_identity(subject.clone());
    linked.identity_attestations.push(claim.clone());
    linked.evidence = row.evidence.clone();
    let mut index = AthleteIdentityIndex::default();
    index.observe(&linked)?;
    index.observe(row)?;
    check!(!index.positive_identity_evidence(&[linked.id.cast(), row.id.cast()]));
    Ok(())
}
