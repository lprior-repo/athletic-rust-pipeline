use super::*;
use crate::read::JournalNamespace;
use crate::{JournalReadBudget, ReadBudget, MAX_JOURNAL_VALUE_BYTES};
use serde::Deserialize;

#[derive(Deserialize)]
struct Shape<'a> {
    #[serde(rename = "type", borrow)]
    kind: Option<&'a RawValue>,
    chunks: Option<usize>,
    total_bytes: Option<usize>,
    #[serde(borrow)]
    data: Option<&'a RawValue>,
}

struct Inventory<'a> {
    grant: &'a mut AttachmentReplayGrantV1,
    root_key: Vec<u8>,
    seen: usize,
    chunks: usize,
}

pub(super) fn budget(
    view: &impl RetainedJournalReadSource,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<JournalReadBudget>> {
    let root_key = root_key(schema, key, grant)?;
    let mut inventory = Inventory {
        grant,
        root_key,
        seen: 0,
        chunks: 0,
    };
    let contents = view.scan_journal_namespace(
        JournalNamespace::new(schema.phase(), key)?,
        ReadBudget {
            rows: 6,
            bytes: 4 * 1024 * 1024,
            record_bytes: MAX_JOURNAL_VALUE_BYTES,
        },
        &mut inventory,
        |state| {
            state
                .grant
                .charge_steps(4)
                .map_err(|error| Error::Domain(error).into())
        },
        Inventory::accept,
    )?;
    if inventory.seen == 0 {
        return Ok(None);
    }
    if inventory.seen
        != inventory
            .chunks
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?
    {
        return Err(Error::Corrupt("external root has missing physical chunks").into());
    }
    let passes = usize::try_from(contents.physical_rows())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(1)
        .and_then(|n| n.checked_mul(17))
        .ok_or(StoreError::CounterOverflow)?;
    inventory
        .grant
        .charge_steps(passes)
        .map_err(Error::Domain)?;
    let measured =
        usize::try_from(contents.encoded_bytes()).map_err(|_| StoreError::CounterOverflow)?;
    JournalReadBudget::new(measured.min(super::super::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES))
        .map(Some)
}

impl Inventory<'_> {
    fn accept(&mut self, key: &[u8], raw: &[u8]) -> StoreResult<()> {
        self.grant.charge_steps(1).map_err(Error::Domain)?;
        if self.seen == 0 {
            if key != self.root_key.as_slice() {
                return Err(Error::Corrupt("external root prefix has a foreign root").into());
            }
            let shape: Shape<'_> =
                serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
                    key: "external root physical inventory".to_owned(),
                    source,
                })?;
            if shape.kind.is_some() {
                self.chunks = shape
                    .chunks
                    .ok_or(Error::Corrupt("external manifest chunk count"))?;
                if self.chunks == 0 || self.chunks > 5 {
                    return Err(Error::Corrupt("external manifest chunk bounds").into());
                }
                self.grant
                    .charge_bytes(
                        shape
                            .total_bytes
                            .ok_or(Error::Corrupt("external manifest bytes"))?,
                    )
                    .map_err(Error::Domain)?;
            }
        } else {
            self.accept_chunk(key, raw)?;
        }
        self.seen = self
            .seen
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }

    fn accept_chunk(&mut self, key: &[u8], raw: &[u8]) -> StoreResult<()> {
        let ordinal = self
            .seen
            .checked_sub(1)
            .ok_or(StoreError::CounterOverflow)?;
        if ordinal >= self.chunks {
            return Err(Error::Corrupt("external root has surplus physical chunks").into());
        }
        let size = self
            .root_key
            .len()
            .checked_add(5)
            .ok_or(StoreError::CounterOverflow)?;
        self.grant.charge_bytes(size).map_err(Error::Domain)?;
        if key != crate::journal::chunk_key(&self.root_key, ordinal)?.as_slice() {
            return Err(
                Error::Corrupt("external root has foreign or missing physical chunks").into(),
            );
        }
        let shape: Shape<'_> =
            serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
                key: "external chunk physical inventory".to_owned(),
                source,
            })?;
        if let Some(data) = shape.data {
            self.grant
                .charge_bytes(data.get().len())
                .map_err(Error::Domain)?;
        }
        Ok(())
    }
}

fn root_key(
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Corrupt("external root descriptor bounds").into());
    }
    let size = schema
        .phase()
        .len()
        .checked_add(key.len())
        .and_then(|n| n.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    grant.charge_bytes(size).map_err(Error::Domain)?;
    let mut root_key = Vec::new();
    root_key
        .try_reserve_exact(size)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    root_key.extend_from_slice(schema.phase().as_bytes());
    root_key.push(0);
    root_key.extend_from_slice(key.as_bytes());
    Ok(root_key)
}
