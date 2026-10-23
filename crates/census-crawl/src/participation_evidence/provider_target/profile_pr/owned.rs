use super::{invalid, read_source_rows, SourceProfilePrV1};
use census_domain::model::{SourceIdentity, SourceNamespace};
use census_store::{Store, StoreResult};
use chrono::{DateTime, Utc};

pub struct OwnedProfilePrFactsV1 {
    source: SourceIdentity,
    profile_url: String,
    body_sha256: String,
    acquired_at: DateTime<Utc>,
    rows: Vec<SourceProfilePrV1>,
}

impl OwnedProfilePrFactsV1 {
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }

    pub fn profile_url(&self) -> &str {
        &self.profile_url
    }

    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }

    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }

    pub fn rows(&self) -> &[SourceProfilePrV1] {
        &self.rows
    }

    pub fn into_rows(self) -> Vec<SourceProfilePrV1> {
        self.rows
    }
}

pub fn read_owned_profile_capture(bytes: &[u8]) -> StoreResult<OwnedProfilePrFactsV1> {
    let packet = Store::decode_public_profile_capture_v1(bytes)?;
    let capture = packet.capture();
    let profile_url = capture.work().request().original_url();
    let athlete_id = packet.athlete_id()?;
    let acquired_at = DateTime::parse_from_rfc3339(capture.response().observed_at())
        .map_err(|_| invalid("owned profile PR observation timestamp invalid"))?
        .with_timezone(&Utc);
    let body = std::str::from_utf8(capture.body())
        .map_err(|_| invalid("owned profile PR body is not UTF-8"))?;
    let rows = read_source_rows(body, profile_url, packet.site(), athlete_id)?
        .into_iter()
        .flatten()
        .collect();
    Ok(OwnedProfilePrFactsV1 {
        source: SourceIdentity::new(SourceNamespace::MilesplitAthlete, athlete_id)
            .with_url(profile_url.to_owned()),
        profile_url: profile_url.to_owned(),
        body_sha256: capture.prepared().content_digest().as_str().to_owned(),
        acquired_at,
        rows,
    })
}
