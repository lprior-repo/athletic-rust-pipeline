use super::*;
use census_domain::model::{
    AttestationQualification, CanonicalAthlete, Evidence, IdentityAttestation, SourceIdentity,
    SourceNamespace, SourceObservation, SourceRef,
};
use census_domain::public_identity_history::BindingPartition;
use sha2::{Digest, Sha256};

pub(super) struct Controlled {
    pub candidates: [AthleteCandidateId; 3],
    pub public: [PublicAthleteId; 3],
    pub partitions: Vec<BindingPartition>,
    pub affected: Vec<String>,
}

pub(super) fn install(store: &Store, writer: &PublicIdentityWriterV1) -> TestResult<Controlled> {
    let athletes = store.snapshot().scan::<CanonicalAthlete>(Table::Athletes)?;
    let [original] = athletes.as_slice() else {
        return Err("one controlled reviewed-roster athlete".into());
    };
    check!(eq; original.grad_year.get(), 2027);
    check!(eq; original.canonical_name, "Controlled Runner");
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted original identity")?;
    let PublicResolution::Canonical(first) = accepted.resolve_candidate(&original.id.cast())?
    else {
        return Err("controlled roster canonical birth".into());
    };
    let aliases = [
        alias(
            original,
            SourceNamespace::TfrrsAthlete,
            "990000001",
            "Controlled Synthetic Identity Alias",
        )?,
        alias(
            original,
            SourceNamespace::DirectAthleticsAthlete,
            "990000002",
            "Controlled Synthetic Direct Alias",
        )?,
    ];
    store.append_many(Table::Athletes, &aliases)?;
    let [middle, last] = &aliases;
    let candidates = [original.id.cast(), middle.id.cast(), last.id.cast()];
    let public = [
        first.public_id,
        birth(store, writer, middle)?,
        birth(store, writer, last)?,
    ];
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("three births")?;
    let mut partitions = Vec::new();
    for (candidate, id) in candidates.iter().zip(&public) {
        let bindings = accepted.component_bindings(id)?;
        check!(eq; bindings.len(), 1);
        partitions.push(BindingPartition {
            representative: candidate.clone(),
            binding_ids: bindings.into_iter().collect(),
        });
    }
    partitions.sort_by(|left, right| left.binding_ids.cmp(&right.binding_ids));
    attest(store, original.clone(), aliases)?;
    let affected = affected(store, &candidates)?;
    Ok(Controlled {
        candidates,
        public,
        partitions,
        affected,
    })
}

fn alias(
    original: &CanonicalAthlete,
    namespace: SourceNamespace,
    source_id: &str,
    name: &str,
) -> TestResult<CanonicalAthlete> {
    let mut added = CanonicalAthlete::new(
        &original.school,
        name,
        original.grad_year,
        original.gender,
        SourceIdentity::new(namespace, source_id),
    )?;
    added.evidence.push(Evidence::parsed(
        SourceRef::new(
            "controlled synthetic identity fixture",
            Some(format!(
                "https://controlled-identity.test/athletes/{source_id}"
            )),
        ),
        "2026-10-16",
    ));
    Ok(added)
}

fn birth(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    athlete: &CanonicalAthlete,
) -> TestResult<PublicAthleteId> {
    let birth = writer.establish_source_bound(
        store,
        &format!("controlled-roster-alias-birth:{}", athlete.id),
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    check!(!birth.replayed());
    Ok(birth.public_id().clone())
}

fn attest(
    store: &Store,
    mut original: CanonicalAthlete,
    aliases: [CanonicalAthlete; 2],
) -> TestResult {
    let [mut middle, mut last] = aliases;
    let middle_source = middle
        .source
        .as_ref()
        .ok_or("controlled provider subject B")?
        .clone();
    let last_source = last
        .source
        .as_ref()
        .ok_or("controlled provider subject C")?
        .clone();
    original.add_identity(middle_source.clone());
    original
        .identity_attestations
        .push(claim(&middle_source, "controlled-r3-roster")?);
    middle
        .identity_attestations
        .push(claim(&middle_source, "controlled-r3-tfrrs")?);
    middle.add_identity(last_source.clone());
    middle
        .identity_attestations
        .push(claim(&last_source, "controlled-r3-direct-bridge")?);
    last.identity_attestations
        .push(claim(&last_source, "controlled-r3-direct")?);
    store.append_many(Table::Athletes, &[original, middle, last])?;
    Ok(())
}

pub(super) fn additional_review_evidence(store: &Store, controlled: &Controlled) -> TestResult {
    let mut rows = store.snapshot().scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; rows.len(), 3);
    let original = rows
        .iter_mut()
        .find(|row| row.id.as_str() == controlled.candidates[0].as_str())
        .ok_or("controlled rereview original candidate")?;
    let b_source = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "990000001");
    original
        .identity_attestations
        .push(claim(&b_source, "controlled-r3-independent-rereview")?);
    store.append_many(Table::Athletes, &rows)?;
    Ok(())
}

fn claim(subject: &SourceIdentity, publisher: &str) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(&json!({
        "controlled_test_fixture": true,
        "not_original_source_or_production_human_approval": true,
        "subject": subject,
        "publisher": publisher
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(bytes)),
        acquired_at: "2026-10-16T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: publisher.into(),
        subject_locator: "subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}

fn affected(store: &Store, candidates: &[AthleteCandidateId; 3]) -> TestResult<Vec<String>> {
    let snapshot = store.snapshot();
    check!(snapshot
        .scan::<census_domain::model::CanonicalPerformance>(Table::Performances)?
        .is_empty());
    check!(snapshot
        .scan::<census_domain::model::RelayResult>(Table::RelayResults)?
        .is_empty());
    let observations = snapshot.scan::<SourceObservation>(Table::SourceObservations)?;
    check!(eq; observations.len(), 1);
    check!(observations
        .iter()
        .all(|row| matches!(row, SourceObservation::School(_))));
    let mut keys = candidates
        .iter()
        .map(|id| format!("athletes:{id}"))
        .collect::<Vec<_>>();
    keys.sort();
    check!(eq; keys.len(), 3);
    Ok(keys)
}
