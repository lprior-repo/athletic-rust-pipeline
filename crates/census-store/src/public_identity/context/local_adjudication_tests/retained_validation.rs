use super::*;
use crate::public_identity::{
    graph_reference::GraphAuthorityReferenceV1, graph_witness::manual::rebuild_manual,
    retained::Live,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV3;

#[test]
fn retained_affected_validator_rejects_omitted_and_replaced_record_keys() -> TestResult {
    let fixture = merged()?;
    let (record, graph) = retained(&fixture)?;
    validate(&fixture, &graph, &record)?;
    for key in &record.request.affected_record_keys {
        let mut omitted = record.clone();
        omitted
            .request
            .affected_record_keys
            .retain(|held| held != key);
        assert!(matches!(
            validate(&fixture, &graph, &omitted),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "graph manual affected keys"
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
            validate(&fixture, &graph, &replaced),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "graph manual affected keys"
            )))
        ));
    }
    Ok(())
}

#[test]
fn retained_affected_validator_rejects_foreign_snapshot_store_and_authority_bindings() -> TestResult
{
    let fixture = merged()?;
    let (record, graph) = retained(&fixture)?;
    validate(&fixture, &graph, &record)?;
    let changed = [
        LocalIdentityAdjudicationRecordV3 {
            snapshot_sequence: record
                .snapshot_sequence
                .checked_add(1)
                .ok_or("sequence overflow")?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV3 {
            evidence_generation: record
                .evidence_generation
                .checked_add(1)
                .ok_or("evidence overflow")?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV3 {
            derived_generation: record
                .derived_generation
                .checked_add(1)
                .ok_or("derived overflow")?,
            ..record.clone()
        },
        LocalIdentityAdjudicationRecordV3 {
            store: StoreIdentity::parse(&different_digest(record.store.as_str()))?,
            ..record.clone()
        },
    ];
    for changed in changed {
        assert!(matches!(
            validate(&fixture, &graph, &changed),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "manual complete capture binding"
            )))
        ));
    }
    Ok(())
}

fn retained(
    fixture: &ReviewedFixture,
) -> TestResult<(LocalIdentityAdjudicationRecordV3, GraphAuthorityReferenceV1)> {
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-retained-semantic-validation",
        request(fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    Ok((prepared.record().clone(), prepared.graph().clone()))
}

fn validate(
    fixture: &ReviewedFixture,
    graph: &GraphAuthorityReferenceV1,
    record: &LocalIdentityAdjudicationRecordV3,
) -> crate::StoreResult<()> {
    let snapshot = fixture.store.db.snapshot();
    let reader = Live {
        snapshot: &snapshot,
        entities: &fixture.store.entities,
        meta: &fixture.store.meta,
        journal: &fixture.store.journal,
        receipts: &fixture.store.receipts,
    };
    let ledger = fixture.writer.verified(&fixture.store)?;
    rebuild_manual(
        &reader,
        graph,
        record,
        ledger.history.projection(),
        ledger.history.records(),
    )?;
    Ok(())
}

#[test]
fn retained_graph_validator_rejects_changed_domain_reference() -> TestResult {
    let fixture = merged()?;
    let (mut record, graph) = retained(&fixture)?;
    let other = stage_inventory(&fixture, "different-retained-graph")?;
    record.graph = other.graph().domain_reference()?;
    check!(matches!(
        validate(&fixture, &graph, &record),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "graph reference domain binding"
        )))
    ));
    Ok(())
}

fn different_digest(original: &str) -> String {
    let first = "0".repeat(64);
    if first == original {
        "1".repeat(64)
    } else {
        first
    }
}
