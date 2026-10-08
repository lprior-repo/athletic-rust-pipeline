use super::super::{
    budget::Footprint, run::receipt::PARSER_REVISION, ResultSetOptions, ResultSetRequest,
    RESULT_SET_PHASE,
};
use crate::{AdapterContext, CrawlError, CrawlResult};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::ops::Range;
use std::path::PathBuf;

mod filesystem;
pub(in crate::milesplit::results) mod replay;

const INPUT_PHASE: &str = "milesplit_result_requests_v1";
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_WORK: usize = 4_000_000;
const BUFFER_BYTES: usize = 32 * 1024;

type Generation = (
    String,
    String,
    String,
    census_domain::model::SchoolYear,
    String,
    chrono::NaiveDate,
);

#[derive(Serialize, Deserialize)]
struct Manifest {
    input_file: String,
    input_digest: String,
    generation_digest: String,
    bytes: usize,
    request_count: usize,
}

pub(super) struct Archived {
    path: PathBuf,
    manifest: Manifest,
}

struct Requests<'a>(&'a [ResultSetRequest]);

impl Serialize for Requests<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        self.0.iter().try_for_each(|request| {
            sequence.serialize_element(&(&request.url, request.jurisdiction))
        })?;
        sequence.end()
    }
}

pub(super) fn archive(
    ctx: &AdapterContext<'_>,
    options: &ResultSetOptions,
) -> CrawlResult<Archived> {
    admit_inputs(ctx, options)?;
    let generation = (
        INPUT_PHASE,
        RESULT_SET_PHASE,
        PARSER_REVISION,
        ctx.school_year,
        &ctx.observed_on,
        ctx.performance_as_of,
    );
    let input = (&generation, Requests(&options.urls));
    let footprint = Footprint::of(&input)?;
    limit("original request encoded bytes", footprint.bytes, MAX_BYTES)?;
    limit(
        "original request serialization work",
        footprint.work,
        MAX_WORK,
    )?;
    let directory = ctx
        .fetcher
        .cache_dir()
        .join("archive")
        .join("milesplit-request-inputs");
    let archived = filesystem::archive(
        &directory,
        &input,
        filesystem::digest(&generation)?,
        options.urls.len(),
    )?;
    replay::range(
        &archived.locator(0..options.urls.len(), options.urls.len())?,
        |ordinal, request| {
            let original = options
                .urls
                .get(ordinal)
                .ok_or_else(|| invariant("archived request ordinal exceeds supplied input"))?;
            if original != &request {
                return Err(invariant(
                    "archived request differs from exact supplied input",
                ));
            }
            Ok(())
        },
    )?;
    Ok(archived)
}

fn admit_inputs(ctx: &AdapterContext<'_>, options: &ResultSetOptions) -> CrawlResult<()> {
    limit("original request count", options.urls.len(), MAX_WORK / 4)?;
    let bytes = options
        .urls
        .iter()
        .try_fold(ctx.observed_on.len(), |bytes, request| {
            bytes
                .checked_add(request.url.len())
                .ok_or_else(|| invariant("original input byte count overflow"))
        })?;
    limit(
        "request archive directory bytes",
        ctx.fetcher.cache_dir().as_os_str().len(),
        4096,
    )?;
    limit("original request source bytes", bytes, MAX_BYTES)
}

impl Archived {
    pub(super) fn locator(&self, range: Range<usize>, pending: usize) -> CrawlResult<String> {
        if range.start > range.end || range.end > self.manifest.request_count {
            return Err(invariant(
                "original request locator range exceeds committed input",
            ));
        }
        let path = std::fs::canonicalize(&self.path).map_err(|source| CrawlError::Io {
            path: self.path.clone(),
            source,
        })?;
        let mut url = url::Url::from_file_path(path)
            .map_err(|()| invariant("original request manifest path is not a file URL"))?;
        url.set_fragment(Some(&format!(
            "requests={}..{};pending={pending};input_sha256={};generation_sha256={}",
            range.start, range.end, self.manifest.input_digest, self.manifest.generation_digest
        )));
        let locator = url.to_string();
        limit("original request locator bytes", locator.len(), 16 * 1024)?;
        Ok(locator)
    }
}

fn limit(resource: &'static str, requested: usize, maximum: usize) -> CrawlResult<()> {
    if requested > maximum {
        return Err(CrawlError::Resource {
            resource,
            requested,
            limit: maximum,
        });
    }
    Ok(())
}

fn invariant(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.into(),
    }
}
