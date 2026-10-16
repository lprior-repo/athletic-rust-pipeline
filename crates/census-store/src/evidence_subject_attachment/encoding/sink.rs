use super::*;

pub(super) fn measure(value: &impl Serialize, maximum: usize) -> StoreResult<usize> {
    let mut sink = Sink {
        bytes: 0,
        maximum,
        output: None,
        failure: None,
    };
    let result = write_serialized(&mut sink, value);
    if let Some(error) = sink.failure {
        return Err(error);
    }
    result?;
    Ok(sink.bytes)
}

pub(super) fn encode(value: &impl Serialize, bytes: usize) -> StoreResult<Vec<u8>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    let mut sink = Sink {
        bytes: 0,
        maximum: bytes,
        output: Some(output),
        failure: None,
    };
    let result = write_serialized(&mut sink, value);
    if let Some(error) = sink.failure {
        return Err(error);
    }
    result?;
    if sink.bytes != bytes {
        return Err(Error::Corrupt("object encoding changed after measurement").into());
    }
    sink.output
        .ok_or_else(|| Error::Corrupt("object encoding output absent").into())
}

struct Sink {
    bytes: usize,
    maximum: usize,
    output: Option<Vec<u8>>,
    failure: Option<StoreError>,
}

impl Sink {
    fn append(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let needed = self
            .bytes
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        if needed > self.maximum {
            return Err(Error::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource: "object encoded bytes",
                needed,
                available: self.maximum,
            })
            .into());
        }
        if let Some(output) = &mut self.output {
            output.extend_from_slice(bytes);
        }
        self.bytes = needed;
        Ok(())
    }
}

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failure.is_some() {
            return Err(io::Error::other("attachment object encoder poisoned"));
        }
        match self.append(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(io::Error::other("attachment object encoding refused"))
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
