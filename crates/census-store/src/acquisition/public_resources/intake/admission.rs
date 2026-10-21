use super::{codec, refused, Mint, PublicSchoolIntakePacketV1};
use crate::acquisition::{admission::ReadMeter, qualification, AcquisitionReadBudget};
use crate::{CaptureError, Receipt, StoreError, StoreResult};
use fjall::{Keyspace, Readable, Snapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicSchoolIntakeAdmissionV1 {
    mint_bytes: Vec<u8>,
    receipt_bytes: Vec<u8>,
}

impl PublicSchoolIntakeAdmissionV1 {
    pub fn from_retained(
        packet: &PublicSchoolIntakePacketV1,
        mint_bytes: &[u8],
        receipt_bytes: &[u8],
    ) -> StoreResult<Self> {
        let mint: Mint = codec::decode(mint_bytes)?;
        let receipt: Receipt = codec::decode(receipt_bytes)?;
        mint.validate(packet, &receipt)?;
        Ok(Self {
            mint_bytes: owned(mint_bytes)?,
            receipt_bytes: owned(receipt_bytes)?,
        })
    }

    pub fn mint_bytes(&self) -> &[u8] {
        &self.mint_bytes
    }
    pub fn receipt_bytes(&self) -> &[u8] {
        &self.receipt_bytes
    }
}

pub(crate) fn read(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    packet: &PublicSchoolIntakePacketV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<PublicSchoolIntakeAdmissionV1> {
    let format = crate::format::read_capture_format(snapshot, meta, budget)?;
    let qualification = qualification::declaration_admitted(
        snapshot,
        meta,
        journal,
        &format,
        &mut ReadMeter::Recovery(budget),
    )?
    .ok_or(CaptureError::CaptureCapabilityMissing)?;
    if packet.directory().work().slot().store_identity() != qualification.store_identity() {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    let mint = Mint::from_packet(packet)?;
    let mint_bytes = raw(snapshot, meta, mint.key().as_bytes(), budget)?;
    let operation = format!("{}{}", super::OPERATION_PREFIX, mint.packet.as_str());
    let receipt_bytes = raw(snapshot, receipts, operation.as_bytes(), budget)?;
    PublicSchoolIntakeAdmissionV1::from_retained(packet, &mint_bytes, &receipt_bytes)
}

pub(super) fn raw(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Vec<u8>> {
    budget.admission_probe()?;
    let length = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| refused("public school intake admission record absent"))?;
    let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if length > codec::MAX_BYTES || key.len() > crate::MAX_JOURNAL_KEY_BYTES {
        return Err(CaptureError::RecordTooLarge.into());
    }
    budget.public_owner_record(key.len(), length)?;
    let bytes = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| refused("public school intake admission record disappeared"))?;
    crate::acquisition::codec::reserve_decode(&bytes, budget)?;
    budget.reserve_recovery_owned(length)?;
    owned(&bytes)
}

fn owned(bytes: &[u8]) -> StoreResult<Vec<u8>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes.len())
        .map_err(|_| refused("public school intake admission allocation"))?;
    output.extend_from_slice(bytes);
    Ok(output)
}
