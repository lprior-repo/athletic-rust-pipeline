use super::*;

#[test]
fn public_inspection_workflow_derives_private_token_encoding() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    check!(eq; inputs.store_identity(), &store.store_identity()?);
    let review = RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("controlled-public-enrollment")?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Explicit controlled test review")?,
    };
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, review)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_enrollment_v1(prepared, &approved, &digest)?;
    check!(applied.written());
    Ok(())
}

#[test]
fn public_historical_replay_ignores_later_input_generations() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let command = command(&store)?;
    let operation = command.operation.as_str().to_owned();
    let prepared = store.prepare_roster_source_enrollment_command_v1(store.snapshot().revision(), command)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let first = store.apply_roster_source_enrollment_v1(prepared, &approved, &digest)?;
    let next = store.generations.next_entities()?;
    let mut batch = store.db.batch();
    crate::generation::write_entities(&mut batch, &store.meta, next);
    batch.commit()?;
    store.generations.publish_entities(next);
    let before = crate::public_identity_tests::support::retained_state(&store)?;
    let replay = store.replay_roster_source_enrollment_v1(&operation, &digest)?;
    check!(eq; replay.receipt(), first.receipt());
    check!(eq; crate::public_identity_tests::support::retained_state(&store)?, before);
    check!(store.replay_roster_source_enrollment_v1("changed", &digest).is_err());
    Ok(())
}

fn review() -> Result<RosterEnrollmentReviewV1, Box<dyn std::error::Error + Send + Sync>> {
    Ok(RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("controlled-public-review")?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Controlled explicit review")?,
    })
}

fn advance_generation(store: &Store) -> TestResult {
    let next = store.generations.next_entities()?;
    let mut batch = store.db.batch();
    crate::generation::write_entities(&mut batch, &store.meta, next);
    batch.commit()?;
    store.generations.publish_entities(next);
    Ok(())
}

#[test]
fn foreign_inspection_and_stale_preparation_refuse() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let (_other_directory, other, _other_writer) = crate::public_identity_tests::support::fixture()?;
    let foreign = other.inspect_roster_source_enrollment_v1()?;
    check!(store.prepare_roster_source_enrollment_v1(foreign, review()?).is_err());
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    advance_generation(&store)?;
    check!(store.prepare_roster_source_enrollment_v1(inputs, review()?).is_err());
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    Ok(())
}

#[test]
fn stale_prepared_public_enrollment_cannot_commit() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, review()?)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    advance_generation(&store)?;
    check!(store.apply_roster_source_enrollment_v1(prepared, &approved, &digest).is_err());
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    Ok(())
}
