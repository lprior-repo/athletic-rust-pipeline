use super::DecisionWrite;
use crate::public_identity::{codec, PublicIdentityError};
use crate::{Store, StoreError, StoreResult, Table};
use fjall::OwnedWriteBatch;

impl DecisionWrite<'_> {
    pub(super) fn stage(self, store: &Store, batch: &mut OwnedWriteBatch) -> StoreResult<usize> {
        match self {
            Self::Accepted(decision) => {
                let rows = store
                    .count(Table::AthleteIdentityDecisions)?
                    .checked_add(1)
                    .ok_or(StoreError::CounterOverflow)?;
                crate::batch::refuse_over_bound(Table::AthleteIdentityDecisions, rows)?;
                let key = crate::keys::observation_key(
                    Table::AthleteIdentityDecisions,
                    &decision.id,
                    crate::keys::DERIVED_SEQUENCE,
                );
                if store
                    .entities
                    .contains_key(&key)
                    .map_err(|source| StoreError::Read { source })?
                {
                    return Err(PublicIdentityError::Corrupt("preexisting decision").into());
                }
                batch.insert(&store.entities, key, codec::encode(decision, 65_536)?);
                store.put_row_mark(batch, Table::AthleteIdentityDecisions, rows);
                Ok(65_536)
            }
            Self::RetainedInWitness => {
                Ok(crate::public_identity::local_adjudication::MAX_WITNESS_BYTES)
            }
        }
    }
}
