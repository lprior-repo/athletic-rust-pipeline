use super::{codec, read, refused, Entry};
use crate::acquisition::public_resources::{intake, read as public};
use crate::{AcquisitionReadBudget, PublicResourceCaptureOwnerV1, Receipt, Store, StoreResult};
use census_domain::model::AcquisitionOccurrenceRefV1;
use fjall::Readable;

mod encode;
mod packet;
pub(crate) mod startup;

pub use packet::PublicProfileCapturePacketV1;
pub(crate) const PHASE: &str = "public_resource_child_profile_capture_v1";
pub(crate) const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_SCAN_BYTES: u64 = 17_592_186_044_416;
const INTERPRETATION: &str = "milesplit_owned_public_profile_capture_v1";

pub(crate) fn scan_budget() -> StoreResult<crate::ReadBudget> {
    let profiles = census_domain::UsJurisdiction::ALL
        .len()
        .checked_mul(
            usize::try_from(super::MAX_PROFILES_PER_OWNER)
                .map_err(|_| crate::StoreError::CounterOverflow)?,
        )
        .ok_or(crate::StoreError::CounterOverflow)?;
    scan_budget_for(profiles)
}

pub(super) fn scan_budget_for(profiles: usize) -> StoreResult<crate::ReadBudget> {
    let profiles = u64::try_from(profiles).map_err(|_| crate::StoreError::CounterOverflow)?;
    let rows = profiles
        .checked_mul(crate::evidence_subject_attachment::MAX_RETAINED_CAPTURE_PHYSICAL_ROWS)
        .ok_or(crate::StoreError::CounterOverflow)?;
    let bytes = profiles
        .checked_mul(u64::try_from(MAX_BYTES).map_err(|_| crate::StoreError::CounterOverflow)?)
        .ok_or(crate::StoreError::CounterOverflow)?;
    Ok(crate::ReadBudget {
        rows,
        bytes: bytes.min(MAX_SCAN_BYTES),
        record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
    })
}

impl Store {
    pub fn decode_public_profile_capture_v1(
        bytes: &[u8],
    ) -> StoreResult<PublicProfileCapturePacketV1> {
        PublicProfileCapturePacketV1::decode(bytes)
    }

    pub fn retain_public_profile_capture_v1(
        &self,
        owner: &PublicResourceCaptureOwnerV1,
        original: &AcquisitionOccurrenceRefV1,
    ) -> StoreResult<PublicProfileCapturePacketV1> {
        let _writes = self.lock_appends();
        let mut budget = AcquisitionReadBudget::for_work();
        let declaration = owner.declaration(self, &mut budget)?;
        let capture = intake::startup::capture_original(self, owner, original)?;
        let snapshot = self.db.snapshot();
        let child = read::entry(
            &snapshot,
            &self.meta,
            &self.journal,
            &self.receipts,
            &declaration,
            read::key(capture.work())?.as_bytes(),
            &mut budget,
        )?;
        let receipt: Receipt = public::record(
            &snapshot,
            &self.receipts,
            child.operation()?.as_bytes(),
            &mut budget,
        )?
        .ok_or_else(|| refused("public profile capture child receipt absent"))?;
        let packet = PublicProfileCapturePacketV1::new(declaration, child, receipt, capture)?;
        self.commit_public_profile_capture(&packet)?;
        Ok(packet)
    }

    fn commit_public_profile_capture(
        &self,
        packet: &PublicProfileCapturePacketV1,
    ) -> StoreResult<()> {
        let bytes = packet.encode()?;
        let key = packet.reference()?;
        if let Some(held) = self
            .snapshot_locked()
            .read_public_profile_capture_v1(key.as_str())?
        {
            if held.encode()? != bytes {
                return Err(refused("public profile capture retained packet differs"));
            }
            return crate::acquisition::codec::repeat(self);
        }
        let operation = operation(key.as_str());
        if self
            .db
            .snapshot()
            .contains_key(&self.receipts, operation.as_bytes())
            .map_err(|source| crate::StoreError::Read { source })?
        {
            return Err(refused("public profile capture orphan receipt"));
        }
        let receipt = Receipt {
            operation,
            digest: key.as_str().to_owned(),
            at: chrono::Utc::now().to_rfc3339(),
            appended: 0,
            credited: false,
        };
        require_receipt(packet, &receipt)?;
        let records = intake::startup::records_for(PHASE, key.as_str(), &receipt.at, &bytes)?;
        let mut batch = self.db.batch();
        for (key, value) in records {
            batch.insert(&self.journal, key, value);
        }
        batch.insert(
            &self.receipts,
            receipt.operation.as_bytes(),
            codec::encode(&receipt)?,
        );
        crate::acquisition::codec::commit(batch)
    }
}

pub(crate) fn operation(key: &str) -> String {
    format!("{PHASE}:{key}")
}

pub(crate) fn require_receipt(
    packet: &PublicProfileCapturePacketV1,
    receipt: &Receipt,
) -> StoreResult<()> {
    let at = chrono::DateTime::parse_from_rfc3339(&receipt.at)
        .map_err(|_| refused("public profile capture receipt time"))?;
    let observed = chrono::DateTime::parse_from_rfc3339(packet.capture().response().observed_at())
        .map_err(|_| refused("public profile capture observed time"))?;
    let key = packet.reference()?;
    if receipt.operation != operation(key.as_str())
        || receipt.digest != key.as_str()
        || receipt.appended != 0
        || receipt.credited
        || at < observed
    {
        return Err(refused("public profile capture receipt differs"));
    }
    Ok(())
}
