use super::codec::{checked_add, corrupt, count, encoding_error, FrozenCodecResult};
use super::stream::StreamKind;
use census_domain::model::serialized_bytes;
use census_store::Entity;
use sha2::{Digest, Sha256};

pub struct LegacyLogicalDigest {
    hasher: Sha256,
    position: usize,
    open: Option<StreamKind>,
    first_in_stream: bool,
}

impl Default for LegacyLogicalDigest {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyLogicalDigest {
    pub fn new() -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"{");
        Self {
            hasher,
            position: 0,
            open: None,
            first_in_stream: true,
        }
    }

    pub fn begin(&mut self, stream: StreamKind) -> FrozenCodecResult<()> {
        if self.open.is_some() {
            return Err(corrupt(
                "legacy logical digest",
                "a legacy stream is already open",
            ));
        }
        let index = stream.index();
        if index != self.position {
            return Err(corrupt(
                "legacy logical digest",
                &format!(
                    "legacy stream order: {index} cannot follow {}",
                    self.position
                ),
            ));
        }
        if self.position > 0 {
            self.hasher.update(b",");
        }
        if stream == StreamKind::CoverageRows {
            self.hasher.update(b"\"coverage_evidence\":{");
            self.hasher.update(b"\"rows\"");
        } else {
            self.hasher.update(&quoted(stream.wire())?);
        }
        self.hasher.update(b":");
        self.hasher
            .update(if stream.keyed_by_id() { b"{" } else { b"[" });
        self.open = Some(stream);
        self.first_in_stream = true;
        Ok(())
    }

    pub fn push<T: Entity>(&mut self, stream: StreamKind, payload: &T) -> FrozenCodecResult<()> {
        if self.open != Some(stream) {
            return Err(corrupt(
                "legacy logical digest",
                "a legacy push landed outside its open stream",
            ));
        }
        if !self.first_in_stream {
            self.hasher.update(b",");
        }
        self.first_in_stream = false;
        if stream.keyed_by_id() {
            self.hasher.update(&quoted(payload.entity_id())?);
            self.hasher.update(b":");
        }
        self.hasher
            .update(&serialized_bytes(payload).map_err(encoding_error)?);
        Ok(())
    }

    pub fn close(&mut self, stream: StreamKind) -> FrozenCodecResult<()> {
        if self.open != Some(stream) {
            return Err(corrupt(
                "legacy logical digest",
                "a legacy stream closed outside its own bounds",
            ));
        }
        self.hasher
            .update(if stream.keyed_by_id() { b"}" } else { b"]" });
        if stream == StreamKind::CoachLanes {
            self.hasher.update(b"}");
        }
        self.open = None;
        let closed = checked_add("legacy stream position", count(self.position)?, 1)?;
        self.position = usize::try_from(closed).map_err(|_| {
            corrupt(
                "legacy logical digest",
                "the stream position exceeded usize",
            )
        })?;
        Ok(())
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn finish(mut self) -> FrozenCodecResult<String> {
        if self.open.is_some() {
            return Err(corrupt(
                "legacy logical digest",
                "a legacy stream is still open",
            ));
        }
        if self.position != StreamKind::ALL.len() {
            return Err(corrupt(
                "legacy logical digest",
                &format!(
                    "the legacy digest needs all {} streams but closed {}",
                    StreamKind::ALL.len(),
                    self.position
                ),
            ));
        }
        self.hasher.update(b"}");
        Ok(format!("{:x}", self.hasher.finalize()))
    }
}

fn quoted(value: &str) -> FrozenCodecResult<Vec<u8>> {
    serialized_bytes(value).map_err(encoding_error)
}
