mod aliases;
mod application;
mod digests;
mod index;
mod projection;
mod validation;

pub use application::{AcceptedAthleteIdentity, IdentityApplication};
pub use index::{AthleteIdentityIndex, IdentityError};
pub use projection::{AthleteIdentityProjection, IdentityProjectionBuilder};

pub(crate) use digests::{athlete_identity_digest, identity_verdict_digest};

use census_domain::model::AppliedAthleteIdentity;
use fjall::PersistMode;
use std::collections::HashSet;

use crate::batch::refuse_over_bound;
use crate::keys::{observation_key, DERIVED_SEQUENCE};
use crate::{Store, StoreError, StoreResult, Table, MAX_ID_BYTES};

pub const MAX_IDENTITY_APPLICATION_BATCH: usize = 1024;

impl Store {
    pub fn apply_identity_decisions(
        &self,
        decisions: &[AcceptedAthleteIdentity],
    ) -> StoreResult<u64> {
        if decisions.len() > MAX_IDENTITY_APPLICATION_BATCH {
            return Err(StoreError::Refused {
                detail: format!(
                    "identity application batch exceeds {MAX_IDENTITY_APPLICATION_BATCH} records"
                ),
            });
        }
        let _appends = self.lock_appends();
        let mut batch = self.db.batch();
        let mut seen = HashSet::with_capacity(decisions.len());
        let mut added = 0_u64;
        for decision in decisions {
            let record = decision.record();
            if !seen.insert(record.id.as_str()) {
                continue;
            }
            let key = identity_key(record)?;
            if self.identity_already_applied(&key, record)? {
                continue;
            }
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing an accepted identity application".to_owned(),
                source,
            })?;
            batch.insert(&self.entities, key, value);
            added = added.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        }
        if added == 0 {
            return Ok(0);
        }
        let rows = self
            .count(Table::AthleteIdentityDecisions)?
            .checked_add(added)
            .ok_or(StoreError::CounterOverflow)?;
        refuse_over_bound(Table::AthleteIdentityDecisions, rows)?;
        self.put_row_mark(&mut batch, Table::AthleteIdentityDecisions, rows);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(added)
    }

    fn identity_already_applied(
        &self,
        key: &[u8],
        record: &AppliedAthleteIdentity,
    ) -> StoreResult<bool> {
        let Some(value) = self
            .entities
            .get(key)
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(false);
        };
        let mut held: AppliedAthleteIdentity =
            serde_json::from_slice(&value).map_err(|source| StoreError::Decode {
                key: record.id.clone(),
                source,
            })?;
        held.members
            .sort_unstable_by(|left, right| left.subject.cmp(&right.subject));
        if same_application(&held, record) {
            Ok(true)
        } else {
            Err(StoreError::Invariant {
                detail: format!(
                    "identity application {} cannot change its payload",
                    record.id
                ),
            })
        }
    }
}

fn identity_key(record: &AppliedAthleteIdentity) -> StoreResult<Vec<u8>> {
    if record.id.is_empty() || record.id.len() > MAX_ID_BYTES {
        return Err(StoreError::Refused {
            detail: "identity application id is empty or exceeds the key bound".to_owned(),
        });
    }
    Ok(observation_key(
        Table::AthleteIdentityDecisions,
        &record.id,
        DERIVED_SEQUENCE,
    ))
}

fn same_application(left: &AppliedAthleteIdentity, right: &AppliedAthleteIdentity) -> bool {
    left.id == right.id
        && left.policy == right.policy
        && left.kind == right.kind
        && left.members == right.members
        && left.canonical_id == right.canonical_id
        && left.case_id == right.case_id
        && left.verdict_digest == right.verdict_digest
}
