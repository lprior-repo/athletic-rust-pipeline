use crate::PublicIdentityError;
use census_domain::model::{AthleteIdentityIndex, CanonicalAthlete};

use super::support::{accepted, refused};
use super::*;

#[test]
fn public_identity_shared_candidate_bucket_refuses_either_valid_source_candidate() -> TestResult {
    let first = subject("shared-candidate-bucket", "701")?;
    let second = subject("shared-candidate-bucket", "702")?;
    check!(ne; first.id, second.id);
    check!(eq; first.candidate_key(), second.candidate_key());
    check!(eq; first.candidate_key().index_id(), second.candidate_key().index_id());
    check!(ne; first.source, second.source);
    for athletes in [[first.clone(), second.clone()], [second, first]] {
        refuse_competing_sources(athletes)?;
    }
    Ok(())
}

#[test]
fn public_identity_shared_primary_source_refuses_either_distinct_valid_candidate() -> TestResult {
    let first = subject("shared-primary-first", "703")?;
    let second = subject("shared-primary-second", "703")?;
    check!(ne; first.id, second.id);
    check!(ne; first.candidate_key(), second.candidate_key());
    check!(ne; first.candidate_key().index_id(), second.candidate_key().index_id());
    check!(first.source.is_some());
    check!(eq; first.source, second.source);
    for athletes in [[first.clone(), second.clone()], [second, first]] {
        refuse_competing_sources(athletes)?;
    }
    Ok(())
}

fn refuse_competing_sources(athletes: [CanonicalAthlete; 2]) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    for athlete in &athletes {
        let mut isolated = AthleteIdentityIndex::default();
        isolated.observe(athlete)?;
        check!(isolated.isolated_source(&athlete.id.cast()));
        let accepted = accepted(athlete)?;
        check!(matches!(accepted.record().members.as_slice(), [member]
            if member.subject.as_str() == athlete.id.as_str()));
    }
    store.append_many(Table::Athletes, &athletes)?;
    let index = store.athlete_identity_index()?;
    check!(eq; index.subjects().count(), 2);
    let head = writer.head(&store)?;
    check!(eq; retained_state(&store)?.len(), 4);
    for (position, athlete) in athletes.iter().enumerate() {
        let before = retained_state(&store)?;
        let operation = format!("competing-source-{position}");
        check!(!index.isolated_source(&athlete.id.cast()));
        let error = refused(writer.establish_source_bound(
            &store,
            &operation,
            &athlete.id.cast(),
            store.snapshot().revision(),
            &head,
        ))?;
        check!(matches!(
            error,
            StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
        ));
        let staged = retained_state(&store)?;
        super::support::assert_graph_staging(
            &store,
            &before,
            &staged,
            writer.store_identity(),
            &operation,
        )?;
        let retry = refused(writer.establish_source_bound(
            &store,
            &operation,
            &athlete.id.cast(),
            store.snapshot().revision(),
            &head,
        ))?;
        check!(matches!(
            retry,
            StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
        ));
        check!(eq; retained_state(&store)?, staged);
        check!(eq; writer.head(&store)?, head);
        check!(eq; store.count(Table::AthleteIdentityDecisions)?, 0);
    }
    Ok(())
}
