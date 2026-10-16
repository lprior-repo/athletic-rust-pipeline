use super::*;
use census_store::paged_generation::{AuthorityTableV2, RawTableReferenceV2, TableBindingV2};
use census_store::{Entity, GraphAuthorityReferenceV1, PublicIdentityInputRevisionsV2};

type PhysicalRows = Vec<(Vec<u8>, Vec<u8>)>;

pub(super) struct Input {
    evidence_generation: u64,
    derived_generation: u64,
    revisions: Vec<u64>,
    athletes: PhysicalRows,
    observations: PhysicalRows,
}

impl Input {
    pub(super) fn capture(snapshot: &StoreSnapshot<'_>) -> TestResult<Self> {
        let PublicIdentityInputRevisionsV2::Enabled(revisions) =
            snapshot.public_identity_input_revisions_v2()?
        else {
            return Err("genuine acceptance requires current input revision enrollment".into());
        };
        let revisions = [
            Table::Athletes,
            Table::SourceObservations,
            Table::ReviewCases,
            Table::IdentityVerdicts,
            Table::Performances,
            Table::RelayResults,
        ]
        .into_iter()
        .map(|table| revisions.table_revision(table))
        .collect::<Result<Vec<_>, _>>()?;
        let athletes = physical::<CanonicalAthlete>(snapshot, Table::Athletes)?;
        let observations = physical::<SourceObservation>(snapshot, Table::SourceObservations)?;
        check!(eq; athletes.len(), 1);
        check!(eq; observations.len(), 4);
        Ok(Self {
            evidence_generation: snapshot.evidence_generation(),
            derived_generation: snapshot.derived_generation(),
            revisions,
            athletes,
            observations,
        })
    }

    fn verify_capture(
        &self,
        graph: &GraphAuthorityReferenceV1,
        snapshot: &StoreSnapshot<'_>,
    ) -> TestResult {
        let capture = serde_json::to_value(graph.capture())?;
        check!(eq; capture.get("store"), Some(&serde_json::to_value(snapshot.store_identity()?)?));
        check!(eq; capture.get("evidence_generation").and_then(Value::as_u64), Some(self.evidence_generation));
        check!(eq; capture.get("derived_generation").and_then(Value::as_u64), Some(self.derived_generation));
        check!(eq; capture.get("table_revisions"), Some(&serde_json::to_value(&self.revisions)?));
        Ok(())
    }
}

pub(super) fn verify(
    snapshot: &StoreSnapshot<'_>,
    installed: &installed::Installed,
    outcome: &IdentityCommitOutcomeV1,
    input: &Input,
) -> TestResult<Value> {
    let frozen = snapshot.freeze_public_identity_v2()?;
    let replayed = frozen.replay()?;
    check!(eq; replayed.head(), outcome.head());
    check!(eq; replayed.public_ids().cloned().collect::<Vec<_>>(), vec![outcome.public_id().clone()]);
    let bundle: Value = serde_json::from_slice(frozen.bytes())?;
    let operations = bundle
        .get("operations")
        .and_then(Value::as_array)
        .ok_or("frozen source operation inventory")?;
    let [operation] = operations.as_slice() else {
        return Err("exactly one held-graph source operation required".into());
    };
    check!(eq; operation.get("sequence").and_then(Value::as_u64), Some(1));
    let witness: Value = serde_json::from_str(
        operation
            .get("witness")
            .and_then(Value::as_str)
            .ok_or("actual held-graph source acceptance witness")?,
    )?;
    check!(eq; witness.get("version").and_then(Value::as_u64), Some(8));
    check!(eq; witness.get("operation").and_then(Value::as_str), Some(OPERATION));
    check!(eq; witness.get("record_digest").and_then(Value::as_str), Some(outcome.head().digest.as_str()));
    check!(eq; witness.pointer("/payload/family").and_then(Value::as_str), Some("source"));
    check!(eq; witness.pointer("/payload/candidate").and_then(Value::as_str), Some(installed.candidate.as_str()));
    check!(eq; witness.pointer("/payload/public_id").and_then(Value::as_str), Some(outcome.public_id().as_str()));
    let graph: GraphAuthorityReferenceV1 = serde_json::from_value(
        witness
            .get("graph")
            .ok_or("actual acceptance graph reference")?
            .clone(),
    )?;
    input.verify_capture(&graph, snapshot)?;
    let seal = snapshot.raw_generation_seal(graph.seal(), graph.capture())?;
    check!(eq; seal.binding(), graph.capture());
    check!(eq; seal.declared_inventory().rows(), 5);
    for reference in seal.tables() {
        let binding = graph.capture().table_binding(reference.table())?;
        let actual = graph_rows(snapshot, &binding, reference)?;
        let expected = match reference.table() {
            AuthorityTableV2::Athletes => input.athletes.as_slice(),
            AuthorityTableV2::SourceObservations => input.observations.as_slice(),
            AuthorityTableV2::ReviewCases
            | AuthorityTableV2::IdentityVerdicts
            | AuthorityTableV2::Performances
            | AuthorityTableV2::RelayResults => &[],
        };
        check!(eq; actual.as_slice(), expected);
        check!(eq; reference.inventory().rows(), u64::try_from(expected.len())?);
    }
    Ok(witness)
}

fn graph_rows(
    snapshot: &StoreSnapshot<'_>,
    binding: &TableBindingV2,
    reference: &RawTableReferenceV2,
) -> TestResult<PhysicalRows> {
    let table = snapshot.raw_generation_table(&reference.object(), binding)?;
    reference.check_table(&table)?;
    let mut rows = Vec::new();
    for reference in table.blocks() {
        let block = snapshot.raw_generation_block(&reference.object(), binding)?;
        reference.check_block(&block)?;
        for reference in block.pages() {
            let page = snapshot.raw_generation_page(&reference.object(), binding)?;
            reference.check_page(&page)?;
            rows.extend(
                page.rows()
                    .iter()
                    .map(|row| (row.physical_key().to_vec(), row.raw_value().to_vec())),
            );
        }
    }
    Ok(rows)
}

fn physical<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> TestResult<PhysicalRows> {
    let mut rows = Vec::new();
    snapshot.for_each_physical_observation_bounded::<T>(
        table,
        school_budget(),
        |_, key, raw, _| {
            rows.push((key.to_vec(), raw.to_vec()));
            Ok(())
        },
    )?;
    Ok(rows)
}
