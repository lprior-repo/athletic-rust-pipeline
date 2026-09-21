//! Input bounds for one assessment: candidate/profile counts, identity fields,
//! and search-incompleteness reasons.

use super::{SearchCompleteness, MAX_PROFILES, MAX_SEARCH_REASONS, MAX_SOURCE_TEXT_BYTES};
use crate::domain::candidate::CandidateEvidence;
use crate::model::SourceRecord;
use anyhow::{bail, Result};

pub(super) fn validate_input(
    record: &SourceRecord,
    entries: &[CandidateEvidence<'_>],
    search: &SearchCompleteness,
) -> Result<()> {
    if entries.len() > MAX_PROFILES {
        bail!("candidate evidence exceeds {MAX_PROFILES} entries")
    }
    if entries
        .iter()
        .flat_map(|entry| (*entry).profiles().iter())
        .take(MAX_PROFILES * 2 + 1)
        .count()
        > MAX_PROFILES * 2
    {
        bail!("profile fragments exceed twice the candidate bound")
    }
    let source_name = crate::domain::name::CanonicalName::from_source(record)?;
    entries.iter().try_for_each(|entry| {
        let athlete_id = (*entry).athlete_id();
        if (*entry)
            .profiles()
            .iter()
            .any(|profile| profile.athlete_id != athlete_id)
        {
            bail!("candidate profile evidence has a mismatched athlete ID")
        }
        if (*entry)
            .exclusion()
            .is_some_and(|exclusion| source_name.as_ref() != Some(exclusion.source_name()))
        {
            bail!("name exclusion belongs to another source-name context")
        }
        Ok::<(), anyhow::Error>(())
    })?;
    const IDENTITY_FIELDS: [&str; 5] = [
        "Person First",
        "Person Last",
        "Schools Name",
        "Address Mailing / Permanent City",
        "Address Mailing / Permanent Region",
    ];
    IDENTITY_FIELDS
        .iter()
        .filter_map(|field| record.fields.get(*field))
        .try_for_each(|value| validate_text(value, "identity field"))?;
    if let SearchCompleteness::Incomplete { reasons } = search {
        if reasons.len() > MAX_SEARCH_REASONS {
            bail!("search incompleteness has too many reasons")
        }
        reasons
            .iter()
            .try_for_each(|reason| validate_text(reason, "search reason"))?;
    }
    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<()> {
    if value.len() > MAX_SOURCE_TEXT_BYTES {
        bail!("{field} exceeds {MAX_SOURCE_TEXT_BYTES} bytes")
    }
    if value.chars().any(char::is_control) {
        bail!("{field} contains a control character")
    }
    Ok(())
}
