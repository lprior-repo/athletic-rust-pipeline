use super::*;
use crate::public_identity::correction::ReviewedIdentityRequestV1;

#[test]
fn reviewed_plan_cannot_supply_its_own_authority_or_write_on_refusal() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let request = ReviewedIdentityRequestV1::Revoke {
        case_id: "caller-asserted-correction".into(),
        canonical: public_id()?,
        revoke_merges: vec![1],
        affected_record_keys: vec!["record:1".into()],
    };
    let head = writer.head(&store)?;
    stage_graph(&store, &writer, "unauthorized-revoke")?;
    let before = retained_state(&store)?;
    let result = writer.apply_reviewed_identity(
        &store,
        "unauthorized-revoke",
        &request,
        store.snapshot().revision(),
        &head,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::NotReviewed))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn different_person_cannot_authorize_merge() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = inputs("different_person")?;
    store.append_many(Table::Athletes, &inputs.athletes)?;
    store.replace_many(Table::ReviewCases, &inputs.cases)?;
    store.replace_many(Table::IdentityVerdicts, &inputs.verdicts)?;
    let request = ReviewedIdentityRequestV1::Merge {
        case_id: case_id(&inputs)?.into(),
        canonical: public_id()?,
        members: vec![public_id()?],
    };
    let head = writer.head(&store)?;
    stage_graph(&store, &writer, "wrong-review-kind")?;
    let before = retained_state(&store)?;
    let result = writer.apply_reviewed_identity(
        &store,
        "wrong-review-kind",
        &request,
        store.snapshot().revision(),
        &head,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::NotReviewed))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn genuine_different_person_still_requires_valid_historical_component() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = inputs("different_person")?;
    store.append_many(Table::Athletes, &inputs.athletes)?;
    store.replace_many(Table::ReviewCases, &inputs.cases)?;
    store.replace_many(Table::IdentityVerdicts, &inputs.verdicts)?;
    let request = ReviewedIdentityRequestV1::Revoke {
        case_id: case_id(&inputs)?.into(),
        canonical: public_id()?,
        revoke_merges: vec![1],
        affected_record_keys: Vec::new(),
    };
    stage_graph(&store, &writer, "unknown-component")?;
    let before = retained_state(&store)?;
    let result = writer.apply_reviewed_identity(
        &store,
        "unknown-component",
        &request,
        store.snapshot().revision(),
        &writer.head(&store)?,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::History(
            census_domain::public_identity_history::HistoryError::Unknown("public ID")
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn operation_names_cannot_cross_source_and_reviewed_commits() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("cross-kind", "45")?;
    store.append(Table::Athletes, &athlete)?;
    writer.establish_source_bound(
        &store,
        "immutable-name",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let request = ReviewedIdentityRequestV1::Revoke {
        case_id: "caller-review".into(),
        canonical: public_id()?,
        revoke_merges: vec![1],
        affected_record_keys: vec!["record:1".into()],
    };
    let before = retained_state(&store)?;
    let result = writer.apply_reviewed_identity(
        &store,
        "immutable-name",
        &request,
        store.snapshot().revision(),
        &writer.head(&store)?,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn historical_v2_refuses_reviewed_fresh_write_without_mutation() -> TestResult {
    historical_writer_refusal(census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityContextV2)
}

fn historical_writer_refusal(
    version: census_domain::public_identity_qualification::PublicIdentityQualificationVersion,
) -> TestResult {
    use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    let declaration = crate::public_identity::init::create_historical_test_fixture(&root, version)?;
    let before = crate::public_identity_tests::support::physical_state(&root)?;
    let accepted = crate::Store::replay_historical_public_identity(&root)?;
    check!(eq; accepted.qualification_version(), version);
    check!(eq; accepted.head(), &census_domain::public_identity_history::genesis(declaration.store_identity())?);
    check!(accepted.history_records().is_empty());
    check!(eq; accepted.public_ids().count(), 0);
    for _ in 0..2 {
        let result = crate::Store::open_public_identity_qualification_v3(&root);
        check!(
            matches!(result, Err(StoreError::PublicIdentity(PublicIdentityError::QualificationVersionMismatch { expected, found }))
            if expected == PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 && found == version)
        );
        let historical = match version {
            PublicIdentityQualificationVersion::SourceBoundV1 => {
                crate::Store::open_public_identity_qualification_v1(&root)
            }
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2 => {
                crate::Store::open_public_identity_qualification_v2(&root)
            }
            PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 => {
                return Err("historical fixture version".into())
            }
        };
        check!(
            matches!(historical, Err(StoreError::PublicIdentity(PublicIdentityError::QualificationVersionMismatch { expected, found }))
            if expected == PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 && found == version)
        );
        check!(eq; crate::public_identity_tests::support::physical_state(&root)?, before);
    }
    Ok(())
}

#[test]
fn historical_fixture_helper_refuses_current_graph_before_path_creation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    let result = crate::public_identity::init::create_historical_test_fixture(&root,
        census_domain::public_identity_qualification::PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "historical test fixture version"
        )))
    ));
    check!(!root.exists());
    Ok(())
}
