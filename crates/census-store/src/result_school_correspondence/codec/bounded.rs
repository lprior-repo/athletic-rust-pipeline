use super::super::*;
use serde::Serialize;

pub(in super::super) fn encode(
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let mut measure = Measure {
        size: 0,
        grant,
        error: None,
    };
    let encoded = serde_json::to_writer(&mut measure, value);
    if let Some(error) = measure.error {
        return Err(grant_error(error));
    }
    encoded.map_err(|_| Error::BudgetExceeded)?;
    let size = measure.size;
    grant.charge_bytes(size).map_err(grant_error)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Error::BudgetExceeded)?;
    serde_json::to_writer(&mut bytes, value)
        .map_err(|_| Error::Corrupt("source closure encoding"))?;
    if bytes.len() != size {
        return Err(Error::Corrupt("source closure size changed").into());
    }
    Ok(bytes)
}

struct Measure<'a> {
    size: usize,
    grant: &'a mut AttachmentReplayGrantV1,
    error: Option<census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError>,
}
impl std::io::Write for Measure<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.size = self
            .size
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("closure size overflow"))?;
        if self.size > census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes
        {
            return Err(std::io::Error::other("closure size limit"));
        }
        if let Err(error) = self.grant.charge_steps(1) {
            self.error = Some(error);
            return Err(std::io::Error::other("closure grant exhausted"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(in super::super) fn require_canonical(
    value: &impl Serialize,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut compare = Compare {
        raw,
        offset: 0,
        grant,
        error: None,
    };
    let encoded = serde_json::to_writer(&mut compare, value);
    if let Some(error) = compare.error {
        return Err(grant_error(error));
    }
    encoded.map_err(|_| Error::Corrupt("noncanonical source closure"))?;
    if compare.offset != raw.len() {
        return Err(Error::Corrupt("source closure trailing bytes").into());
    }
    Ok(())
}

struct Compare<'a, 'g> {
    raw: &'a [u8],
    offset: usize,
    grant: &'g mut AttachmentReplayGrantV1,
    error: Option<census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError>,
}
impl std::io::Write for Compare<'_, '_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Err(error) = self.grant.charge_steps(1) {
            self.error = Some(error);
            return Err(std::io::Error::other("closure comparison grant"));
        }
        let end = self
            .offset
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("closure comparison overflow"))?;
        if self.raw.get(self.offset..end) != Some(bytes) {
            return Err(std::io::Error::other("closure canonical bytes differ"));
        }
        self.offset = end;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
