use super::*;
use crate::public_identity::{
    codec,
    correction::{ReviewedIdentityRequestV1, ReviewedWitness},
    witness,
};
use census_domain::public_identity_history::prepare_next;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

#[test]
fn reviewed_witness_shape_kind_and_lifecycle_are_explicit() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = inputs("different_person")?;
    let accepted = authority::accept_reviewed(&inputs, case_id(&inputs)?, "2026-10-14")?;
    let request = ReviewedIdentityRequestV1::Revoke {
        case_id: case_id(&inputs)?.into(),
        canonical: public_id()?,
        revoke_merges: vec![1],
        affected_record_keys: vec!["record:1".into()],
    };
    let prepared = prepare_next(
        writer.store_identity(),
        &writer.head(&store)?,
        request.authorized(&accepted)?,
        ReplayLimits::SMALL,
    )?;
    let staged = inputs.stage(&store, writer.store_identity())?;
    let held = ReviewedWitness::new(
        "codec-only",
        &request,
        &store.snapshot(),
        &prepared,
        accepted.record(),
        staged.object_id(),
    )?;
    held.validate(
        PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        &prepared,
    )?;
    check!(held
        .validate(PublicIdentityQualificationVersion::SourceBoundV1, &prepared)
        .is_err());
    let raw = codec::encode(&held, 65_536)?;
    check!(matches!(
        crate::public_identity::operation::HeldWitness::decode("codec-only", &raw)?,
        crate::public_identity::operation::HeldWitness::Reviewed(_)
    ));
    check!(serde_json::from_slice::<witness::Witness>(&raw).is_err());
    let value = serde_json::to_value(&held)?;
    for field in [
        "case_id",
        "decision_id",
        "record_digest",
        "request_digest",
        "authority_context",
    ] {
        let mut changed = value.clone();
        changed[field] = serde_json::json!("changed");
        let held: ReviewedWitness = serde_json::from_value(changed)?;
        check!(held
            .validate(
                PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
                &prepared
            )
            .is_err());
    }
    let mut changed = value.clone();
    changed["kind"] = serde_json::json!("Merge");
    let held: ReviewedWitness = serde_json::from_value(changed)?;
    check!(held
        .validate(
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
            &prepared
        )
        .is_err());
    let mut changed = value;
    changed["introduced_public_ids"] = serde_json::json!([public_id()?.as_str()]);
    let held: ReviewedWitness = serde_json::from_value(changed)?;
    check!(held
        .validate(
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
            &prepared
        )
        .is_err());
    check!(eq; writer.head(&store)?.sequence, 0);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 0);
    Ok(())
}

#[test]
fn source_witness_cannot_be_relabelled_as_reviewed() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("source-shape", "43")?;
    store.append(Table::Athletes, &athlete)?;
    writer.establish_source_bound(
        &store,
        "source-shape",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let raw = store
        .journal
        .get(witness::key(1))?
        .ok_or("source witness")?;
    let original: serde_json::Value = serde_json::from_slice(&raw)?;
    for version in [3, 4, 9, 10, 11, 12] {
        let mut changed = original.clone();
        changed["version"] = serde_json::json!(version);
        check!(matches!(
            HeldGraphWitness::decode("relabelled", &serde_json::to_vec(&changed)?),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "graph witness schema binding"
            )))
        ));
    }
    check!(matches!(
        HeldGraphWitness::decode("source", &raw)?.payload,
        Payload::Source { .. }
    ));
    Ok(())
}
