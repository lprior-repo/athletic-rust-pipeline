use super::{RosterPhysicalProjectionRowV1, RosterSourceEvidenceV1};
use crate::{StoreResult, Table};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use serde::ser::{SerializeSeq, Serializer};
use serde::Serialize;

#[derive(Serialize)]
struct Manifest<'a> {
    schema: &'static str,
    store: &'a StoreIdentity,
    snapshot_sequence: u64,
    tables: [Table; 4],
    rows: Rows<'a>,
}

struct Rows<'a>(&'a [RosterPhysicalProjectionRowV1]);

#[derive(Serialize)]
struct Row<'a> {
    table: Table,
    physical_key: &'a [u8],
    encoded_bytes: usize,
    encoded_digest: String,
}

impl Serialize for Rows<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        self.0.iter().try_for_each(|physical| {
            sequence.serialize_element(&Row {
                table: physical.table(),
                physical_key: physical.key(),
                encoded_bytes: physical.bytes().len(),
                encoded_digest: super::super::codec::digest(physical.bytes()),
            })
        })?;
        sequence.end()
    }
}

impl RosterSourceEvidenceV1 {
    pub fn physical_projection_manifest(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<u8>> {
        self.require_original_grant(grant)?;
        let result = encode(self, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

fn encode(
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let rows = evidence.physical_projection(grant)?;
    rows.iter().try_for_each(|row| {
        let hash_bytes = row
            .bytes()
            .len()
            .checked_mul(3)
            .ok_or(super::super::RosterSourceIssuanceStoreError::BudgetExceeded)?;
        grant
            .charge_bytes(hash_bytes)
            .map_err(super::super::source_error)?;
        grant.charge_steps(3).map_err(super::super::source_error)
    })?;
    let manifest = Manifest {
        schema: "roster-physical-projection-inventory-v1",
        store: evidence.store_identity(),
        snapshot_sequence: evidence.snapshot_sequence(),
        tables: [
            Table::Schools,
            Table::Teams,
            Table::Conflicts,
            Table::SourceObservations,
        ],
        rows: Rows(rows),
    };
    let bytes = super::super::codec::encode_with_source_grant(&manifest, grant)?;
    evidence.require_original_grant(grant)?;
    Ok(bytes)
}
