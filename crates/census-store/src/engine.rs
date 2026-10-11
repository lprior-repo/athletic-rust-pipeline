use super::{
    Application, Store, StoreBatch, StoreError, StoreResult, MAX_DIGEST_BYTES, MAX_OPERATION_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineOperation {
    operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineDigest {
    digest: String,
}

fn refuse_blank(field: &'static str, value: &str) -> StoreResult<()> {
    if value.is_empty() || value.bytes().any(|b| b == 0 || b == b'\n') {
        return Err(StoreError::Refused {
            detail: format!("engine {field} must be one non-empty line"),
        });
    }
    Ok(())
}

fn refuse_part(field: &'static str, value: &str) -> StoreResult<()> {
    refuse_blank(field, value)?;
    if value.bytes().any(|b| b == b':') {
        return Err(StoreError::Refused {
            detail: format!("engine {field} must not carry a scope separator"),
        });
    }
    Ok(())
}

fn join(parts: &[&str]) -> StoreResult<String> {
    let mut bytes = 0_usize;
    for part in parts {
        bytes = bytes
            .checked_add(part.len())
            .ok_or(StoreError::CounterOverflow)?
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
    }
    if bytes > MAX_OPERATION_BYTES {
        return Err(StoreError::Refused {
            detail: format!("engine operation of {bytes} bytes exceeds its ceiling"),
        });
    }
    let mut out = String::with_capacity(bytes);
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            out.push(':');
        }
        out.push_str(part);
    }
    Ok(out)
}

impl EngineOperation {
    pub fn new(operation: String) -> StoreResult<Self> {
        refuse_blank("operation", &operation)?;
        if operation.len() > MAX_OPERATION_BYTES {
            return Err(StoreError::Refused {
                detail: format!(
                    "engine operation of {} bytes exceeds its ceiling",
                    operation.len()
                ),
            });
        }
        if !operation.contains(':') {
            return Err(StoreError::Refused {
                detail: "engine operation must name its scope with a separator".to_string(),
            });
        }
        Ok(Self { operation })
    }

    pub fn jurisdiction(state: &str, season: &str, revision: u32) -> StoreResult<Self> {
        refuse_part("state", state)?;
        refuse_part("season", season)?;
        if revision == 0 {
            return Err(StoreError::Refused {
                detail: "engine revision must be at least 1".to_string(),
            });
        }
        Self::new(join(&[
            "jurisdiction",
            state,
            season,
            &revision.to_string(),
        ])?)
    }

    pub fn source_sweep(
        source: &str,
        state: &str,
        season: &str,
        revision: u32,
    ) -> StoreResult<Self> {
        refuse_part("source", source)?;
        refuse_part("state", state)?;
        refuse_part("season", season)?;
        if revision == 0 {
            return Err(StoreError::Refused {
                detail: "engine revision must be at least 1".to_string(),
            });
        }
        Self::new(join(&[
            "source-sweep",
            source,
            state,
            season,
            &revision.to_string(),
        ])?)
    }

    pub fn athlete(source: &str, source_athlete_id: &str, revision: u32) -> StoreResult<Self> {
        refuse_part("source", source)?;
        refuse_part("source_athlete_id", source_athlete_id)?;
        if revision == 0 {
            return Err(StoreError::Refused {
                detail: "engine revision must be at least 1".to_string(),
            });
        }
        Self::new(join(&[
            "athlete",
            source,
            source_athlete_id,
            &revision.to_string(),
        ])?)
    }

    pub fn school(source: &str, source_school_id: &str, revision: u32) -> StoreResult<Self> {
        refuse_part("source", source)?;
        refuse_part("source_school_id", source_school_id)?;
        if revision == 0 {
            return Err(StoreError::Refused {
                detail: "engine revision must be at least 1".to_string(),
            });
        }
        Self::new(join(&[
            "school",
            source,
            source_school_id,
            &revision.to_string(),
        ])?)
    }

    pub fn meet(source: &str, source_meet_id: &str, revision: u32) -> StoreResult<Self> {
        refuse_part("source", source)?;
        refuse_part("source_meet_id", source_meet_id)?;
        if revision == 0 {
            return Err(StoreError::Refused {
                detail: "engine revision must be at least 1".to_string(),
            });
        }
        Self::new(join(&[
            "meet",
            source,
            source_meet_id,
            &revision.to_string(),
        ])?)
    }

    pub fn as_str(&self) -> &str {
        &self.operation
    }
}

impl EngineDigest {
    pub fn new(digest: String) -> StoreResult<Self> {
        refuse_blank("digest", &digest)?;
        if digest.len() > MAX_DIGEST_BYTES {
            return Err(StoreError::Refused {
                detail: format!(
                    "engine digest of {} bytes exceeds its ceiling",
                    digest.len()
                ),
            });
        }
        Ok(Self { digest })
    }

    pub fn as_str(&self) -> &str {
        &self.digest
    }
}

impl Store {
    pub fn engine_commit(
        &self,
        operation: &EngineOperation,
        digest: &EngineDigest,
        stage: impl FnOnce(&mut StoreBatch) -> StoreResult<()>,
    ) -> StoreResult<Application> {
        tracing::debug!(operation = operation.as_str(), "engine commit staged");
        let mut batch = self.write_batch();
        stage(&mut batch)?;
        let outcome = batch.commit_once(operation.as_str(), digest.as_str())?;
        tracing::debug!(
            operation = operation.as_str(),
            written = outcome.written(),
            "engine commit applied"
        );
        Ok(outcome)
    }
}
