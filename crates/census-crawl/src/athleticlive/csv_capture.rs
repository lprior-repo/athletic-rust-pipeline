use super::capture;
use crate::coach_contacts::artifact::ContactCsv;
use crate::net::cache::CacheMeta;
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::CanonicalMeet;

const MAX_CAPTURE_BYTES: usize = 32 * 1024 * 1024;

pub(super) struct FrozenCsv {
    pub(super) csv: ContactCsv,
    pub(super) capture: CsvProvenance,
}

pub(super) struct CsvProvenance {
    pub(super) acquired_on: String,
    url: String,
    digest: String,
}

pub(super) fn open(
    ctx: &AdapterContext<'_>,
    input: &str,
    metadata: Option<&CacheMeta>,
) -> CrawlResult<FrozenCsv> {
    let frozen = capture::freeze(ctx, input, metadata, MAX_CAPTURE_BYTES)?;
    let metadata = metadata.ok_or_else(|| schema(input, "CSV producer metadata is missing"))?;
    let acquired_on = chrono::DateTime::parse_from_rfc3339(&metadata.fetched_at)
        .map_err(|error| schema(input, &error.to_string()))?
        .date_naive()
        .to_string();
    drop(frozen.bytes);
    let csv =
        ContactCsv::open(&frozen.path, 0).map_err(|error| schema(input, &error.to_string()))?;
    let capture = CsvProvenance {
        acquired_on,
        url: metadata.url.clone(),
        digest: metadata.content_digest.clone(),
    };
    Ok(FrozenCsv { csv, capture })
}

impl CsvProvenance {
    pub(super) fn bind(&self, meet: &mut CanonicalMeet) {
        for evidence in &mut meet.evidence {
            evidence.source.url = Some(self.url.clone());
            evidence.note = Some(format!("capture sha256={}", self.digest));
        }
    }
}

fn schema(input: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: input.to_string(),
        detail: detail.to_string(),
    }
}
