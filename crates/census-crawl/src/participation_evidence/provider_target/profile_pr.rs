use super::super::{bound, hash, invalid, markup, RetainedHtmlCaptureV1, MAX_PACKET};
use census_domain::model::{PublicSourceResourceV1, SourceIdentity, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::StoreResult;

mod retained;
mod rows;
mod types;
pub use retained::{read_retained, RetainedProfilePrTableV1};
pub use types::{CheckedProfilePrTableV1, SourceProfilePrV1};

pub const INTERPRETATION: &str = "milesplit_public_profile_declared_pr_table_v1";

pub fn read<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    site: UsJurisdiction,
    athlete_id: &str,
) -> StoreResult<CheckedProfilePrTableV1<'a>> {
    read_if_published(capture, site, athlete_id)?
        .ok_or_else(|| invalid("profile PR table is absent"))
}

pub fn read_if_published<'a>(
    capture: &'a RetainedHtmlCaptureV1,
    site: UsJurisdiction,
    athlete_id: &str,
) -> StoreResult<Option<CheckedProfilePrTableV1<'a>>> {
    let resource = PublicSourceResourceV1::milesplit_athlete_profile(athlete_id)
        .map_err(|_| invalid("profile PR athlete identity invalid"))?;
    let canonical = capture
        .requested_url()
        .strip_suffix("/stats")
        .map_or(capture.requested_url(), std::convert::identity);
    resource
        .validate_target(site, canonical)
        .map_err(|_| invalid("profile PR requested owner differs"))?;
    let body = capture.body()?;
    markup::canonical(&body, canonical)?;
    let tables = markup::elements(&body, b"table", Some("pr-table"))?;
    if tables.is_empty() {
        return Ok(None);
    }
    let table = markup::single(tables)?;
    if table.attribute("id") != Some("prTable") {
        return Err(invalid("profile PR table identity differs"));
    }
    let rows = rows::read(&table)?;
    bound(rows.len(), 256, "profile PR rows")?;
    let reference = hash(&super::super::codec::encode(
        &(INTERPRETATION, site, athlete_id, capture),
        MAX_PACKET,
    )?);
    Ok(Some(CheckedProfilePrTableV1 {
        capture,
        athlete: SourceIdentity::new(SourceNamespace::MilesplitAthlete, athlete_id)
            .with_url(capture.requested_url().to_owned()),
        reference,
        rows,
    }))
}

#[cfg(test)]
mod tests;
