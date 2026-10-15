use super::*;
use crate::public_identity::local_adjudication::affected;
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;

#[test]
fn retained_affected_validator_rejects_omitted_and_replaced_record_keys() -> TestResult {
    let fixture = merged()?;
    let (record, identity, affected) = retained(&fixture)?;
    affected.validate(&identity, &record)?;
    for key in &record.request.affected_record_keys {
        let mut omitted = record.clone();
        omitted
            .request
            .affected_record_keys
            .retain(|held| held != key);
        assert!(matches!(
            affected.validate(&identity, &omitted),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "manual affected record inventory"
            )))
        ));
    }
    for unrelated in results::unrelated_keys(&fixture)? {
        let mut replaced = record.clone();
        *replaced
            .request
            .affected_record_keys
            .first_mut()
            .ok_or("affected key")? = unrelated;
        replaced.request.affected_record_keys.sort();
        assert!(matches!(
            affected.validate(&identity, &replaced),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "manual affected record inventory"
            )))
        ));
    }
    Ok(())
}

#[test]
fn retained_affected_validator_rejects_foreign_snapshot_store_and_authority_bindings() -> TestResult
{
    let fixture = merged()?;
    let (record, identity, affected) = retained(&fixture)?;
    affected.validate(&identity, &record)?;
    let changed = [
        LocalIdentityAdjudicationRecordV2 {
            snapshot_sequence: record
                .snapshot_sequence
                .checked_add(1)
                .ok_or("sequence overflow")?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV2 {
            evidence_generation: record
                .evidence_generation
                .checked_add(1)
                .ok_or("evidence overflow")?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV2 {
            derived_generation: record
                .derived_generation
                .checked_add(1)
                .ok_or("derived overflow")?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV2 {
            store: StoreIdentity::parse(&different_digest(record.store.as_str()))?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV2 {
            authority_context: different_digest(&record.authority_context),
            ..record.clone()
        },
    ];
    for changed in changed {
        assert!(matches!(
            affected.validate(&identity, &changed),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "manual affected snapshot binding"
            )))
        ));
    }
    Ok(())
}

fn retained(
    fixture: &ReviewedFixture,
) -> TestResult<(
    LocalIdentityAdjudicationRecordV2,
    super::super::Inputs,
    affected::Inputs,
)> {
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-retained-semantic-validation",
        request(fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let record = prepared.record().clone();
    let snapshot = fixture.store.db.snapshot();
    let identity = super::super::read_inputs(
        &snapshot,
        &fixture.store.journal,
        &record.authority_context,
        &record.store,
    )?;
    let affected = affected::read(&snapshot, &fixture.store.journal, &record.affected_context)?;
    Ok((record, identity, affected))
}

fn different_digest(original: &str) -> String {
    let first = "0".repeat(64);
    if first == original {
        "1".repeat(64)
    } else {
        first
    }
}
