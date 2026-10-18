use super::{RosterPhysicalProjectionRowV1, RosterSourceEvidenceV1};
use crate::{StoreResult, Table};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use serde::ser::{SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

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

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RetainedManifest {
    schema: String,
    store: StoreIdentity,
    snapshot_sequence: u64,
    tables: [Table; 4],
    rows: Vec<RetainedRow>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RetainedRow {
    table: Table,
    physical_key: Vec<u8>,
    encoded_bytes: usize,
    encoded_digest: String,
}

impl RosterSourceEvidenceV1 {
    pub(in crate::roster_source_issuance) fn retained_physical_projection(
        reference: &census_domain::roster_affiliation_authority::RosterAuthorityObjectRefV1,
        store: &StoreIdentity,
        sequence: u64,
        objects: &std::collections::BTreeMap<String, Vec<u8>>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<RosterPhysicalProjectionRowV1>> {
        use super::super::{codec, source_error, RosterSourceIssuanceStoreError as Error};
        let bytes = objects
            .get(reference.digest().as_str())
            .ok_or(Error::Corrupt("original physical manifest absent"))?;
        if u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)? != reference.bytes().get()
            || codec::digest(bytes) != reference.digest().as_str()
        {
            return Err(Error::InspectedClosureChanged.into());
        }
        grant.charge_bytes(bytes.len()).map_err(source_error)?;
        grant.charge_steps(1).map_err(source_error)?;
        let manifest: RetainedManifest = codec::decode(bytes)?;
        if manifest.schema != "roster-physical-projection-inventory-v1"
            || &manifest.store != store
            || manifest.snapshot_sequence != sequence
            || manifest.tables
                != [
                    Table::Schools,
                    Table::Teams,
                    Table::Conflicts,
                    Table::SourceObservations,
                ]
            || manifest.rows.len() > 4096
        {
            return Err(Error::InspectedClosureChanged.into());
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(manifest.rows.len())
            .map_err(|_| Error::BudgetExceeded)?;
        for row in manifest.rows {
            if !manifest.tables.contains(&row.table) {
                return Err(Error::Corrupt("original physical table differs").into());
            }
            let raw = objects
                .get(&row.encoded_digest)
                .ok_or(Error::Corrupt("original physical row absent"))?;
            if raw.len() != row.encoded_bytes || codec::digest(raw) != row.encoded_digest {
                return Err(Error::InspectedClosureChanged.into());
            }
            grant
                .charge_bytes(
                    raw.len()
                        .checked_add(row.physical_key.len())
                        .ok_or(Error::BudgetExceeded)?,
                )
                .map_err(source_error)?;
            grant.charge_steps(1).map_err(source_error)?;
            rows.push(RosterPhysicalProjectionRowV1::from_checked_parts(
                row.table,
                row.physical_key,
                raw.clone(),
            ));
        }
        Ok(rows)
    }
}
