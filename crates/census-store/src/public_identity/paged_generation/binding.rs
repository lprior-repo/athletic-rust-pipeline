use census_domain::model::{RunManifest, StoreIdentity};
use serde::Serialize;

use super::{invalid, wire, AuthorityTableV2, DigestV2};
use crate::StoreResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct FormatBindingV2(wire::FormatBinding);

impl FormatBindingV2 {
    pub fn new(store_schema: u32, key_format: u32, lifecycle_revision: u32) -> StoreResult<Self> {
        let raw = wire::FormatBinding {
            store_schema,
            key_format,
            lifecycle_revision,
        };
        Self::from_wire(raw)
    }

    fn from_wire(raw: wire::FormatBinding) -> StoreResult<Self> {
        if raw.store_schema == 0 || raw.key_format == 0 || raw.lifecycle_revision == 0 {
            return Err(invalid("zero format revision"));
        }
        Ok(Self(raw))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunBindingV2 {
    Unbound,
    Bound(RunManifest),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturePositionV2 {
    evidence_generation: u64,
    derived_generation: u64,
    snapshot_sequence: u64,
    table_revisions: [u64; 6],
}

impl CapturePositionV2 {
    pub fn new(
        evidence_generation: u64,
        derived_generation: u64,
        snapshot_sequence: u64,
        table_revisions: [u64; 6],
    ) -> Self {
        Self {
            evidence_generation,
            derived_generation,
            snapshot_sequence,
            table_revisions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct CaptureBindingV2(pub(super) wire::CaptureBinding);

impl CaptureBindingV2 {
    pub(in crate::public_identity) fn require_identity_graph_v3_format(&self) -> StoreResult<()> {
        let format = &self.0.format;
        if format.store_schema != 2 || format.key_format != 1
            || !matches!(format.lifecycle_revision, census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE | crate::ROSTER_SOURCE_ISSUANCE_LIFECYCLE | crate::RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE | crate::RESULT_CALENDAR_CORRESPONDENCE_LIFECYCLE)
        {
            return Err(crate::PublicIdentityError::Corrupt("graph authority capture format").into());
        }
        Ok(())
    }

    pub(in crate::public_identity) fn store_identity(&self) -> &StoreIdentity {
        &self.0.store
    }

    pub(in crate::public_identity) fn evidence_generation(&self) -> u64 {
        self.0.evidence_generation
    }

    pub(in crate::public_identity) fn derived_generation(&self) -> u64 {
        self.0.derived_generation
    }

    pub(in crate::public_identity) fn snapshot_sequence(&self) -> u64 {
        self.0.snapshot_sequence
    }

    pub fn new(
        store: StoreIdentity,
        format: FormatBindingV2,
        run: RunBindingV2,
        position: CapturePositionV2,
    ) -> StoreResult<Self> {
        let run = match run {
            RunBindingV2::Unbound => wire::RunBinding::Unbound,
            RunBindingV2::Bound(manifest) => wire::RunBinding::Bound { manifest },
        };
        Self::from_wire(wire::CaptureBinding {
            store,
            format: format.0,
            run,
            evidence_generation: position.evidence_generation,
            derived_generation: position.derived_generation,
            snapshot_sequence: position.snapshot_sequence,
            table_revisions: position.table_revisions,
            authority_policy: 1,
            enrichment_policy: 1,
            projection_policy: 1,
            claim_ordinals: "full-section-v1".into(),
        })
    }

    pub(super) fn from_wire(raw: wire::CaptureBinding) -> StoreResult<Self> {
        FormatBindingV2::from_wire(raw.format.clone())?;
        if raw.authority_policy != 1
            || raw.enrichment_policy != 1
            || raw.projection_policy != 1
            || raw.claim_ordinals != "full-section-v1"
        {
            return Err(invalid("unknown policy or claim ordinal semantics"));
        }
        if let wire::RunBinding::Bound { manifest } = &raw.run {
            if manifest.store_identity != raw.store.as_str() {
                return Err(invalid("run belongs to another store"));
            }
        }
        crate::public_identity::codec::encode(
            &raw,
            usize::try_from(super::MAX_SEAL_JSONL_BYTES)
                .map_err(|_| crate::StoreError::CounterOverflow)?,
        )?;
        Ok(Self(raw))
    }

    pub fn table_binding(&self, table: AuthorityTableV2) -> StoreResult<TableBindingV2> {
        let lineage = DigestV2::of_serialized(&(
            "public-identity-authority-lineage/v2",
            &self.0.store,
            &self.0.format,
            &self.0.run,
        ))?;
        let revision = self
            .0
            .table_revisions
            .get(table.index())
            .copied()
            .ok_or_else(|| invalid("table position"))?;
        let layout = match table {
            AuthorityTableV2::ReviewCases => wire::Layout::DerivedGeneration {
                generation: self.0.derived_generation,
            },
            AuthorityTableV2::IdentityVerdicts => wire::Layout::DerivedMap,
            _ => wire::Layout::ObservationLog,
        };
        Ok(TableBindingV2(wire::TableBinding {
            lineage,
            table,
            revision,
            layout,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct TableBindingV2(pub(super) wire::TableBinding);

impl TableBindingV2 {
    pub fn table(&self) -> AuthorityTableV2 {
        self.0.table
    }

    pub fn digest(&self) -> StoreResult<DigestV2> {
        DigestV2::of_serialized(&("public-identity-authority-table-binding/v2", &self.0))
    }
}
