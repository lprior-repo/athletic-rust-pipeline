use super::{HistoricalRead, Read, Revision, Rows};
use crate::public_identity::{codec, context, inventory, PublicIdentityError};
use crate::{StoreError, StoreResult, Table};
use census_domain::model::StoreIdentity;
use fjall::{Keyspace, Readable, Snapshot};

pub(in crate::public_identity) struct Live<'a> {
    pub snapshot: &'a Snapshot,
    pub entities: &'a Keyspace,
    pub meta: &'a Keyspace,
    pub journal: &'a Keyspace,
    pub receipts: &'a Keyspace,
}

impl Read for Live<'_> {
    fn row(&self, rows: Rows, key: &str) -> StoreResult<Option<Vec<u8>>> {
        match rows {
            Rows::Witness => codec::load(
                self.snapshot,
                self.journal,
                key,
                crate::public_identity::local_adjudication::MAX_WITNESS_BYTES,
            )
            .map(|raw| raw.map(|raw| raw.to_vec())),
            Rows::Receipt => codec::load(self.snapshot, self.receipts, key, 65_536)
                .map(|raw| raw.map(|raw| raw.to_vec())),
            Rows::Decision => self.decision(key),
        }
    }
    fn revision(&self) -> StoreResult<Revision> {
        Ok(Revision {
            evidence_generation: inventory::number(
                self.snapshot,
                self.meta,
                crate::generation::ENTITIES,
            )?,
            derived_generation: inventory::number(
                self.snapshot,
                self.meta,
                crate::generation::DERIVED_CURRENT,
            )?,
            snapshot_sequence: self.snapshot.seqno(),
        })
    }
}

impl HistoricalRead for Live<'_> {
    fn context(&self, root: &str, identity: &StoreIdentity) -> StoreResult<context::Inputs> {
        context::read_inputs(self.snapshot, self.journal, root, identity)
    }
    fn affected_context(
        &self,
        root: &str,
    ) -> StoreResult<crate::public_identity::local_adjudication::affected::Inputs> {
        crate::public_identity::local_adjudication::affected::read(
            self.snapshot,
            self.journal,
            root,
        )
    }
}

impl Live<'_> {
    fn decision(&self, id: &str) -> StoreResult<Option<Vec<u8>>> {
        let key = crate::keys::observation_key(
            Table::AthleteIdentityDecisions,
            id,
            crate::keys::DERIVED_SEQUENCE,
        );
        let size = self
            .snapshot
            .size_of(self.entities, &key)
            .map_err(|source| StoreError::Read { source })?;
        if size.is_some_and(|size| size > 65_536) {
            return Err(PublicIdentityError::Corrupt("oversized accepted decision").into());
        }
        self.snapshot
            .get(self.entities, key)
            .map(|raw| raw.map(|raw| raw.to_vec()))
            .map_err(|source| StoreError::Read { source })
    }
}
