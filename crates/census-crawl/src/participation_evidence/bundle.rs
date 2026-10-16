use super::{
    bound, invalid, json, IndependentParticipationPacketV1, ParticipationEvidenceRefV1,
    VerifiedIndependentParticipationV1, MAX_BUNDLE, MAX_PACKET, MAX_PACKETS,
    PARTICIPATION_EVIDENCE_PHASE,
};
use census_domain::model::StoreIdentity;
use census_store::{JournalReadBudget, StoreError, StoreResult, StoreSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentParticipationBundleV1 {
    version: u32,
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
    entries: BTreeMap<ParticipationEvidenceRefV1, Option<Value>>,
}

impl IndependentParticipationBundleV1 {
    pub fn capture(
        snapshot: &StoreSnapshot<'_>,
        references: &[ParticipationEvidenceRefV1],
    ) -> StoreResult<Self> {
        bound(
            references.len(),
            MAX_PACKETS,
            "independent participation packet count",
        )?;
        let mut entries = BTreeMap::new();
        references.iter().try_for_each(|reference| {
            if entries.contains_key(reference) {
                return Ok(());
            }
            let value = snapshot.journal_payload_bounded(
                PARTICIPATION_EVIDENCE_PHASE,
                reference.as_str(),
                JournalReadBudget::new(MAX_PACKET)?,
            )?;
            entries.insert(reference.clone(), value);
            Ok::<(), StoreError>(())
        })?;
        let bundle = Self {
            version: 1,
            store_identity: snapshot.store_identity()?,
            snapshot_sequence: snapshot.sequence(),
            entries,
        };
        bundle.require_binding(bundle.store_identity.as_str(), snapshot.sequence())?;
        Ok(bundle)
    }

    pub fn require_binding(&self, store_identity: &str, snapshot_sequence: u64) -> StoreResult<()> {
        if self.version != 1
            || self.store_identity.as_str() != store_identity
            || self.snapshot_sequence != snapshot_sequence
            || snapshot_sequence == 0
        {
            return Err(invalid("independent participation bundle binding differs"));
        }
        bound(
            self.entries.len(),
            MAX_PACKETS,
            "independent participation packet count",
        )?;
        let bytes = self
            .entries
            .iter()
            .try_fold(0usize, |total, (reference, payload)| {
                ParticipationEvidenceRefV1::parse(reference.as_str())?;
                let bytes = super::codec::encode(payload, MAX_PACKET)?;
                bound(
                    bytes.len(),
                    MAX_PACKET,
                    "independent participation packet bytes",
                )?;
                total
                    .checked_add(bytes.len())
                    .ok_or(StoreError::CounterOverflow)
            })?;
        bound(bytes, MAX_BUNDLE, "independent participation bundle bytes")
    }

    pub fn read(
        &self,
        reference: &ParticipationEvidenceRefV1,
    ) -> StoreResult<Option<VerifiedIndependentParticipationV1>> {
        self.require_binding(self.store_identity.as_str(), self.snapshot_sequence)?;
        let payload = self
            .entries
            .get(reference)
            .ok_or_else(|| invalid("participation reference was not recorded in frozen closure"))?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let packet: IndependentParticipationPacketV1 =
            serde_json::from_value(payload.clone()).map_err(json)?;
        if packet.reference()? != *reference {
            return Err(invalid("independent participation packet digest differs"));
        }
        packet
            .read(self.store_identity.clone(), self.snapshot_sequence)
            .map(Some)
    }
}
