use super::*;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;

const WRITES_PER_STEP: usize = 32;

pub(super) fn encode(
    value: &impl serde::Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(1).map_err(domain)?;
    let mut measure = Measure {
        bytes: 0,
        writes: 0,
        grant,
        failure: None,
    };
    let result = census_domain::model::write_serialized(&mut measure, value);
    if let Some(error) = measure.failure.take() {
        return Err(domain(error));
    }
    result?;
    let size = measure.bytes;
    let writes = measure.writes;
    grant.charge_bytes(size).map_err(domain)?;
    grant
        .charge_steps(writes.div_ceil(WRITES_PER_STEP))
        .map_err(domain)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| invalid("semantic encoding allocation refused"))?;
    census_domain::model::write_serialized(&mut bytes, value)?;
    if bytes.len() != size {
        return Err(invalid("semantic canonical size changed"));
    }
    Ok(bytes)
}

struct Measure<'a> {
    bytes: usize,
    writes: usize,
    grant: &'a mut AttachmentReplayGrantV1,
    failure: Option<EvidenceSubjectAttachmentError>,
}

impl std::io::Write for Measure<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.writes.is_multiple_of(WRITES_PER_STEP) {
            if let Err(error) = self.grant.charge_steps(1) {
                self.failure = Some(error);
                return Err(std::io::Error::other("semantic measurement grant"));
            }
        }
        self.writes = self
            .writes
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("semantic work overflow"))?;
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("semantic size overflow"))?;
        if self.bytes > LIMIT {
            return Err(std::io::Error::other("semantic size limit"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
