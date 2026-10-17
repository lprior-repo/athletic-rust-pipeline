use super::*;

const SERIALIZER_WRITES_PER_STEP: usize = 32;

pub(super) fn with_grant<T>(
    grant: &mut AttachmentReplayGrantV1,
    read: impl FnOnce(&mut AttachmentReplayGrantV1) -> Result<T>,
) -> Result<T> {
    grant.ensure_available()?;
    let result = read(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn encode(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Vec<u8>> {
    grant.charge_steps(1)?;
    let mut measure = Measure {
        bytes: 0,
        writes: 0,
        grant,
        failure: None,
    };
    let result = crate::model::write_serialized(&mut measure, value);
    if let Some(error) = measure.failure.take() {
        return Err(error.into());
    }
    result?;
    let size = measure.bytes;
    let writes = measure.writes;
    grant.charge_bytes(size)?;
    grant.charge_steps(writes.div_ceil(SERIALIZER_WRITES_PER_STEP))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| ResultSchoolCorrespondenceError::Capacity)?;
    crate::model::write_serialized(&mut bytes, value)?;
    if bytes.len() != size {
        return Err(ResultSchoolCorrespondenceError::Invalid(
            "canonical size changed",
        ));
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
        if self.writes.is_multiple_of(SERIALIZER_WRITES_PER_STEP) {
            if let Err(error) = self.grant.charge_steps(1) {
                self.failure = Some(error);
                return Err(std::io::Error::other("result school measurement grant"));
            }
        }
        self.writes = self
            .writes
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("result school work overflow"))?;
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("result school size overflow"))?;
        if self.bytes > MAX_RESULT_SCHOOL_RECORD_BYTES {
            return Err(std::io::Error::other("result school size limit"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
