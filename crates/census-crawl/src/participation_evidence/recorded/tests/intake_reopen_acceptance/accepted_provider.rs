use super::*;
use crate::participation_evidence::provider_school::{
    ProviderSchoolPacketRefV1,
};
use census_domain::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId, CanonicalAthlete,
    CanonicalSchool, SourceNamespace, SourceObservation,
};
use census_domain::public_identity_history::{
    BindingResolution, CandidateEvidenceKind, EffectiveScope, EstablishmentOrdinal, HistoryRequest,
    PublicResolution, ReplayLimits, ResolvedSuccessor,
};
use census_store::{IdentityCommitOutcomeV1, PublicIdentityError, PublicIdentityWriterV1};

mod graph;
mod identity;
pub(super) mod installed;
use super::provider_school::{fixtures as school_fixtures, observations as scoped_observations};

const OPERATION: &str = "genuine-provider-source-bound-acceptance";

#[test]
fn genuine_provider_source_bound_acceptance_reopens_with_exact_identity_and_originals() -> TestResult
{
    let (directory, store, writer, installed) = installed::install()?;
    let input = graph::Input::capture(&store.snapshot())?;
    let held = store.snapshot();
    let head = writer.head(&store)?;
    let outcome = writer.establish_source_bound(
        &store,
        OPERATION,
        &installed.candidate,
        held.revision(),
        &head,
    )?;
    check!(!outcome.replayed());
    check!(eq; outcome.head().sequence, 1);
    identity::verify(&store, &installed, &outcome)?;
    let witness = graph::verify(&store.snapshot(), &installed, &outcome, &input)?;
    let accepted_state = protected_state(&store)?;
    drop(held);
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    check!(eq; protected_state(&store)?, accepted_state);
    identity::verify(&store, &installed, &outcome)?;
    check!(eq; graph::verify(&store.snapshot(), &installed, &outcome, &input)?, witness);
    let replay = writer.establish_source_bound(
        &store,
        OPERATION,
        &installed.candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(replay.replayed());
    check!(eq; replay.public_id(), outcome.public_id());
    check!(eq; replay.head(), outcome.head());
    check!(eq; protected_state(&store)?, accepted_state);
    installed.verify(&store)?;
    check!(eq; graph::verify(&store.snapshot(), &installed, &outcome, &input)?, witness);
    Ok(())
}

#[test]
fn genuine_provider_already_bound_refusal_preserves_reopened_identity_and_originals() -> TestResult
{
    let (directory, store, writer, installed) = installed::install()?;
    let input = graph::Input::capture(&store.snapshot())?;
    let outcome = writer.establish_source_bound(
        &store,
        OPERATION,
        &installed.candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let witness = graph::verify(&store.snapshot(), &installed, &outcome, &input)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    let before = protected_state(&store)?;
    let result = writer.establish_source_bound(
        &store,
        "genuine-provider-second-establishment",
        &installed.candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    );
    match result {
        Err(StoreError::PublicIdentity(PublicIdentityError::AlreadyBound)) => {}
        other => return Err(format!("expected exact already-bound refusal, got {other:?}").into()),
    }
    check!(eq; protected_state(&store)?, before);
    identity::verify(&store, &installed, &outcome)?;
    check!(eq; graph::verify(&store.snapshot(), &installed, &outcome, &input)?, witness);
    Ok(())
}
