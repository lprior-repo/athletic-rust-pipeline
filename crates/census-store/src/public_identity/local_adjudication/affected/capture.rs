#[cfg(test)]
use super::{allocation, Header, Inputs, SCHEMA};
use super::{invalid, Physical, MAX_KEY_BYTES};
#[cfg(test)]
use crate::public_identity::context::BUDGET;
use crate::StoreResult;
#[cfg(test)]
use crate::{Entity, StoreSnapshot, Table};
use base64::{engine::general_purpose::STANDARD, Engine};
#[cfg(test)]
use census_domain::model::{CanonicalPerformance, RelayResult, StoreIdentity};

#[cfg(test)]
impl Inputs {
    pub(in crate::public_identity) fn capture(
        snapshot: &StoreSnapshot<'_>,
        store: &StoreIdentity,
        authority_context: &str,
    ) -> StoreResult<Self> {
        let performances = collect::<CanonicalPerformance>(snapshot, Table::Performances)?;
        let relays = collect::<RelayResult>(snapshot, Table::RelayResults)?;
        Ok(Self {
            header: Header {
                schema: SCHEMA.into(),
                store: store.clone(),
                authority_context: authority_context.into(),
                evidence_generation: snapshot.evidence_generation(),
                derived_generation: snapshot.derived_generation(),
                snapshot_sequence: snapshot.sequence(),
                counts: [performances.len(), relays.len()],
            },
            performances,
            relays,
        })
    }
}

#[cfg(test)]
fn collect<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> StoreResult<Vec<Physical>> {
    let mut rows = Vec::new();
    snapshot.for_each_physical_observation_bounded(table, BUDGET, |_: T, key, raw, _| {
        let row = Physical::from_raw(key, raw)?;
        rows.try_reserve(1).map_err(allocation)?;
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

impl Physical {
    pub(in crate::public_identity) fn from_raw(key: &[u8], raw: &[u8]) -> StoreResult<Self> {
        if key.len() > MAX_KEY_BYTES {
            return Err(invalid("manual affected physical key bound"));
        }
        let raw = std::str::from_utf8(raw)
            .map_err(|_| invalid("manual affected physical JSON encoding"))?;
        Ok(Self {
            physical_key_base64: STANDARD.encode(key),
            raw_json: raw.to_owned(),
        })
    }
}
