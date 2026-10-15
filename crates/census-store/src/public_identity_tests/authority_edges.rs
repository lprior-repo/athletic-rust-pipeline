use crate::PublicIdentityError;
use census_domain::model::{
    CanonicalAthlete, Grade, ObservedGrade, SchoolYear, SourceAthleteObservation, SourceNamespace,
    SourceObservation, SourceRef,
};
use census_domain::public_identity_history::RegistryHead;

use super::support::refused;
use super::*;

#[test]
fn public_identity_writer_retains_source_only_cohort_contradiction_without_partial_write(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("source-only-contradiction", "501")?;
    check!(!athlete.has_cohort_conflict());
    check!(athlete.observed_grades.is_empty());
    store.append(Table::Athletes, &athlete)?;
    let observation = contradictory_grade()?;
    store.append(Table::SourceObservations, &observation)?;
    check!(!store
        .athlete_identity_index()?
        .isolated_source(&athlete.id.cast()));
    let head = writer.head(&store)?;
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "source-only-contradiction",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &head,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
    ));
    check!(eq; writer.head(&store)?, head);
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?, vec![athlete]);
    check!(eq; store.scan::<SourceObservation>(Table::SourceObservations)?, vec![observation]);
    Ok(())
}

#[test]
fn public_identity_evidence_counter_exhaustion_preserves_exact_replay_and_refuses_new_write(
) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let first = subject("counter-first", "502")?;
    let second = subject("counter-second", "503")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let genesis = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "counter-first",
        &first.id.cast(),
        store.snapshot().revision(),
        &genesis,
    )?;
    store
        .meta
        .insert(crate::generation::ENTITIES, u64::MAX.to_string().as_bytes())?;
    store.flush()?;
    drop(store);
    let root = directory.path().join("identity");
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; store.evidence_generation(), u64::MAX);
    let before = retained_state(&store)?;
    let replayed = writer.establish_source_bound(
        &store,
        "counter-first",
        &first.id.cast(),
        store.snapshot().revision(),
        &genesis,
    )?;
    check!(replayed.replayed());
    check!(eq; replayed.public_id(), committed.public_id());
    check!(eq; replayed.head(), committed.head());
    check!(eq; retained_state(&store)?, before);
    let error = refused(writer.establish_source_bound(
        &store,
        "counter-second",
        &second.id.cast(),
        store.snapshot().revision(),
        committed.head(),
    ))?;
    check!(matches!(error, StoreError::CounterOverflow));
    check!(eq; store.evidence_generation(), u64::MAX);
    check!(eq; writer.head(&store)?, committed.head().clone());
    check!(eq; retained_state(&store)?, before);
    drop(store);
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; store.evidence_generation(), u64::MAX);
    check!(eq; writer.head(&store)?, committed.head().clone());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_exhausted_supplied_head_refuses_without_allocating_an_identity() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("head-overflow", "504")?;
    store.append(Table::Athletes, &athlete)?;
    let head = writer.head(&store)?;
    let supplied = RegistryHead {
        sequence: u64::MAX,
        digest: head.digest.clone(),
    };
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "head-overflow",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &supplied,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::HeadMoved)
    ));
    check!(eq; writer.head(&store)?, head);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

fn contradictory_grade() -> TestResult<SourceObservation> {
    Ok(SourceObservation::Athlete(
        SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            "501",
            "https://mi.milesplit.com/athletes/501",
            "Synthetic source-only contradiction",
            "2026-10-14",
        )
        .with_grade(Some(ObservedGrade {
            grade: Grade::new(10).ok_or("valid fixture grade")?,
            school_year: SchoolYear::new(2025).ok_or("valid fixture school year")?,
            source: SourceRef::new(
                "synthetic retained source grade",
                Some("https://mi.milesplit.com/athletes/501".into()),
            ),
        })),
    ))
}
