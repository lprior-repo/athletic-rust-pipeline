use super::{BATCH_COUNT, MAX_RECORDS, NOTE_BYTES};
use census_domain::model::{normalize_name, CanonicalSchool, Evidence, EvidenceMethod, SourceRef};
use census_domain::UsJurisdiction;
use census_store::{StoreError, StoreResult};
use sha2::{Digest, Sha256};

fn note(index: usize) -> StoreResult<String> {
    let index = u64::try_from(index).map_err(|_| StoreError::CounterOverflow)?;
    let mut note = String::new();
    note.try_reserve_exact(NOTE_BYTES)
        .map_err(|error| StoreError::Refused {
            detail: format!("allocating bounded evidence note: {error}"),
        })?;
    (0..NOTE_BYTES / 32).try_for_each(|block| -> StoreResult<()> {
        let block = u64::try_from(block).map_err(|_| StoreError::CounterOverflow)?;
        let mut hash = Sha256::new();
        hash.update(index.to_le_bytes());
        hash.update(block.to_le_bytes());
        hash.finalize().iter().for_each(|byte| {
            note.push(char::from((byte % 95).saturating_add(32)));
        });
        Ok(())
    })?;
    Ok(note)
}

pub(super) fn school(index: usize) -> StoreResult<CanonicalSchool> {
    if index >= MAX_RECORDS {
        return Err(StoreError::Refused {
            detail: "school index exceeds the ENOSPC record budget".to_string(),
        });
    }
    let name = format!("School_{index:04}");
    let (mut school, _) =
        CanonicalSchool::new(UsJurisdiction::Kansas, &name, normalize_name(&name));
    school.evidence.push(Evidence {
        source: SourceRef::id(format!("drill:{index}")),
        method: EvidenceMethod::Fetched,
        observed_on: "2026-01-01".into(),
        note: Some(note(index)?),
    });
    Ok(school)
}

pub(super) fn schools(start: usize, count: usize) -> StoreResult<Vec<CanonicalSchool>> {
    let end = start
        .checked_add(count)
        .ok_or(StoreError::CounterOverflow)?;
    if count == 0 || count > BATCH_COUNT || end > MAX_RECORDS {
        return Err(StoreError::Refused {
            detail: "school batch exceeds the ENOSPC record budget".to_string(),
        });
    }
    let mut schools = Vec::new();
    schools
        .try_reserve_exact(count)
        .map_err(|error| StoreError::Refused {
            detail: format!("allocating bounded school batch: {error}"),
        })?;
    (start..end).try_for_each(|index| -> StoreResult<()> {
        schools.push(school(index)?);
        Ok(())
    })?;
    Ok(schools)
}

pub(super) fn digest(schools: &[CanonicalSchool]) -> StoreResult<String> {
    let mut hash = Sha256::new();
    schools.iter().try_for_each(|school| -> StoreResult<()> {
        let bytes = serde_json::to_vec(school).map_err(|source| StoreError::Json {
            detail: "serializing ENOSPC evidence digest".to_string(),
            source,
        })?;
        hash.update(bytes);
        Ok(())
    })?;
    Ok(format!("{:x}", hash.finalize()))
}
