use super::*;
use census_domain::model::{
    AttestationQualification, CanonicalAthlete, Evidence, IdentityAttestation, SourceIdentity,
    SourceNamespace, SourceObservation, SourceRef,
};
use census_domain::public_identity_history::BindingPartition;
use sha2::{Digest, Sha256};

pub(super) struct Controlled {
    pub candidates: [AthleteCandidateId; 2],
    pub public: [PublicAthleteId; 2],
    pub partitions: Vec<BindingPartition>,
    pub affected: Vec<String>,
}

pub(super) fn install(store: &Store, writer: &PublicIdentityWriterV1) -> TestResult<Controlled> {
    let athletes = store.snapshot().scan::<CanonicalAthlete>(Table::Athletes)?;
    let [genuine] = athletes.as_slice() else {
        return Err("one genuine pre-control athlete".into());
    };
    check!(eq; genuine.grad_year.get(), 2026);
    check!(eq; genuine.canonical_name, "Zacchaeus Brocks");
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted genuine identity")?;
    let PublicResolution::Canonical(first) = accepted.resolve_candidate(&genuine.id.cast())? else {
        return Err("genuine canonical birth".into());
    };
    let source = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "990000001");
    let mut added = CanonicalAthlete::new(
        &genuine.school,
        "Controlled Synthetic Identity Alias",
        genuine.grad_year,
        genuine.gender,
        source,
    )?;
    added.evidence.push(Evidence::parsed(
        SourceRef::new(
            "controlled synthetic identity fixture",
            Some("https://controlled-identity.test/athletes/990000001".into()),
        ),
        "2026-10-16",
    ));
    store.append_many(Table::Athletes, std::slice::from_ref(&added))?;
    let birth = writer.establish_source_bound(
        store,
        "controlled-documentary-alias-birth",
        &added.id.cast(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    check!(!birth.replayed());
    let candidates = [genuine.id.cast(), added.id.cast()];
    let public = [first.public_id, birth.public_id().clone()];
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("two births")?;
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
    attest(store, genuine.clone(), added)?;
    let affected = affected(store, &candidates)?;
    Ok(Controlled {
        candidates,
        public,
        partitions,
        affected,
    })
}

fn attest(store: &Store, mut genuine: CanonicalAthlete, mut added: CanonicalAthlete) -> TestResult {
    let source = added
        .source
        .as_ref()
        .ok_or("controlled provider subject")?
        .clone();
    genuine.add_identity(source.clone());
    genuine
        .identity_attestations
        .push(claim(&source, "controlled-r3-roster")?);
    added
        .identity_attestations
        .push(claim(&source, "controlled-r3-timer")?);
    store.append_many(Table::Athletes, &[genuine, added])?;
    Ok(())
}

pub(super) fn additional_review_evidence(store: &Store, controlled: &Controlled) -> TestResult {
    let mut rows = store.snapshot().scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; rows.len(), 2);
    let [first, _] = &controlled.candidates;
    let genuine = rows
        .iter_mut()
        .find(|row| row.id.as_str() == first.as_str())
        .ok_or("controlled rereview genuine candidate")?;
    let subject = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "990000001");
    genuine
        .identity_attestations
        .push(claim(&subject, "controlled-r3-independent-rereview")?);
    store.append_many(Table::Athletes, &rows)?;
    Ok(())
}

fn claim(subject: &SourceIdentity, publisher: &str) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(&json!({"controlled_test_fixture":true,
        "not_genuine_source_or_production_human_approval":true, "subject":subject, "publisher":publisher}))?;
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

fn affected(store: &Store, candidates: &[AthleteCandidateId; 2]) -> TestResult<Vec<String>> {
    let snapshot = store.snapshot();
    check!(snapshot
        .scan::<census_domain::model::CanonicalPerformance>(Table::Performances)?
        .is_empty());
    check!(snapshot
        .scan::<census_domain::model::RelayResult>(Table::RelayResults)?
        .is_empty());
    let observations = snapshot.scan::<SourceObservation>(Table::SourceObservations)?;
    check!(eq; observations.len(), 4);
    let mut keys = candidates
        .iter()
        .map(|id| format!("athletes:{id}"))
        .collect::<Vec<_>>();
    let mut athlete_observations = 0usize;
    for observation in &observations {
        if let SourceObservation::Athlete(row) = observation {
            check!(eq; row.namespace, SourceNamespace::MilesplitAthlete);
            check!(eq; row.source_athlete_id, "7223777");
            keys.push(format!("source_observations:{}", observation.id()));
            athlete_observations = athlete_observations
                .checked_add(1)
                .ok_or("observation count")?;
        }
    }
    check!(eq; athlete_observations, 3);
    keys.sort();
    check!(eq; keys.len(), 5);
    Ok(keys)
}
