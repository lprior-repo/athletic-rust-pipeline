use super::*;
use std::io::Write;

const MAX_METADATA: usize = 64 * 1024;

pub(super) fn encode(
    objects: &Path,
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<SourceBindingObjectV1> {
    grant.charge_bytes(MAX_METADATA).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    let mut output = Bounded(Vec::new());
    serde_json::to_writer(&mut output, value).map_err(|source| StoreError::Decode {
        key: "reviewed roster publication metadata".into(),
        source,
    })?;
    let bytes = u64::try_from(output.0.len()).map_err(|_| StoreError::CounterOverflow)?;
    let digest = proof_objects::install(objects, &output.0)?;
    Ok(SourceBindingObjectV1 { digest, bytes })
}

pub(super) fn read(
    objects: &Path,
    reference: &SourceBindingObjectV1,
    ceiling: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<Vec<u8>> {
    if reference.bytes == 0
        || reference.bytes > ceiling
        || !super::super::note::digest(&reference.digest)
    {
        return Err(invalid("reviewed roster retained object reference is invalid").into());
    }
    grant
        .charge_bytes(usize::try_from(reference.bytes).map_err(|_| StoreError::CounterOverflow)?)
        .map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    let bytes = proof_objects::read(objects, &reference.digest, reference.bytes)?;
    if u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)? != reference.bytes {
        return Err(invalid("reviewed roster retained object length differs").into());
    }
    Ok(bytes)
}

struct Bounded(Vec<u8>);

impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let needed = self
            .0
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("reviewed roster metadata size overflow"))?;
        if needed > MAX_METADATA {
            return Err(std::io::Error::other(
                "reviewed roster metadata exceeds limit",
            ));
        }
        self.0
            .try_reserve(bytes.len())
            .map_err(std::io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
