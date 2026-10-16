use super::*;
use crate::ReadBudget;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    operation: String,
    digest: String,
    at: String,
    appended: u64,
    credited: bool,
}

pub(super) fn load(
    view: &impl RetainedJournalReadSource,
    records: &[ReviewedReferenceRecordV1],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<Receipt>> {
    let mut slots = Vec::new();
    for _ in records {
        grant.charge_steps(1).map_err(Error::Domain)?;
        material::allocate(&mut slots)?;
        slots.push(None);
    }
    grant.charge_steps(2).map_err(Error::Domain)?;
    view.visit_receipt_prefix(
        OPERATION_PREFIX.as_bytes(),
        ReadBudget {
            rows: 4096,
            bytes: 4 * 1024 * 1024,
            record_bytes: 2048,
        },
        grant,
        |grant| {
            grant
                .charge_steps(1)
                .map_err(|source| Error::Domain(source).into())
        },
        |grant, key, raw| accept(records, &mut slots, key, raw, grant),
    )?;
    let mut receipts = Vec::new();
    for slot in slots {
        grant.charge_steps(1).map_err(Error::Domain)?;
        material::allocate(&mut receipts)?;
        receipts.push(slot.ok_or(Error::Corrupt("missing attachment receipt"))?);
    }
    Ok(receipts)
}

fn accept(
    records: &[ReviewedReferenceRecordV1],
    slots: &mut [Option<Receipt>],
    key: &[u8],
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_bytes(raw.len()).map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.first() != Some(&b'{') {
        return Err(Error::Corrupt("attachment receipt must be an object").into());
    }
    let receipt: Wire =
        serde_json::from_slice(raw).map_err(|source| Error::Domain(source.into()))?;
    if receipt.operation.as_bytes() != key || receipt.appended != 0 || receipt.credited {
        return Err(Error::Corrupt("attachment receipt metadata").into());
    }
    for (index, record) in records.iter().enumerate() {
        grant.charge_steps(1).map_err(Error::Domain)?;
        if keys::receipt(&record.command.operation)? != receipt.operation {
            continue;
        }
        if receipt.digest != record.record_digest.as_str()
            || receipt.at != record.committed_at.date_naive().to_string()
        {
            return Err(Error::Corrupt("attachment record and receipt differ").into());
        }
        let slot = slots.get_mut(index).ok_or(StoreError::CounterOverflow)?;
        if slot.is_some() {
            return Err(Error::Corrupt("duplicate attachment receipt").into());
        }
        *slot = Some(Receipt {
            operation: receipt.operation,
            digest: receipt.digest,
            at: receipt.at,
            appended: 0,
            credited: false,
        });
        return Ok(());
    }
    Err(Error::Corrupt("orphan attachment receipt").into())
}
