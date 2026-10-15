use super::invalid;
use census_crawl::milesplit::{OwnedAcquisitionRef, RawMetadataArchiveRef, ResultSetRef};
use census_crawl::net::FetchOutcome;
use census_domain::model::{Evidence, EvidenceMethod};
use census_store::{StoreError, StoreResult};
use serde::Deserialize;
use serde_json::Value;

const ROLE: &str = "raw result file meet/season metadata only";
const MAX_NOTE_BYTES: usize = 64 * 1024;

pub(super) struct SourceNotes {
    pub(super) reference: ResultSetRef,
    pub(super) owned: OwnedAcquisitionRef,
    pub(super) raw: RawMetadataArchiveRef,
    pub(super) locator: String,
    pub(super) owned_capture: FetchOutcome,
    pub(super) raw_capture: FetchOutcome,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    url: String,
    response_url: Option<String>,
    method: String,
    status: u16,
    content_digest: String,
    bytes: usize,
    fetched_at: String,
    content_type: Option<String>,
    request_body: Value,
    request_headers: Vec<Value>,
}

pub(super) fn value(evidence: &Evidence) -> StoreResult<Option<Value>> {
    let Some(note) = &evidence.note else {
        return Ok(None);
    };
    if note.len() > MAX_NOTE_BYTES {
        return Err(invalid("participation source note exceeds 64 KiB"));
    }
    Ok(serde_json::from_str(note).ok())
}

pub(super) fn metadata(value: &Value) -> bool {
    keys(
        value,
        &[
            "role",
            "meet_id",
            "result_set_id",
            "meet_name",
            "meet_date",
            "region",
            "sport",
            "school_year",
            "owned_capture",
            "raw_metadata_capture",
            "projection_revision",
            "parser_revision",
        ],
    ) && value.get("role").and_then(Value::as_str) == Some(ROLE)
}

pub(super) fn owned(value: &Value) -> bool {
    keys(
        value,
        &[
            "locator",
            "result_id",
            "meet_id",
            "result_set_id",
            "team_id",
            "source_athlete",
            "cohort",
            "published_grad_year",
            "provider",
            "capture_url",
            "sha256",
            "acquired_at",
            "raw_metadata_url",
            "meet_name",
            "meet_date",
            "sport",
            "school_year",
            "identity_decision",
            "best_mark_scope",
        ],
    ) && value.get("identity_decision").and_then(Value::as_str)
        == Some("none; source-owned observation only")
        && value.get("best_mark_scope").and_then(Value::as_str)
            == Some("observed performance; not lifetime PR")
}

impl SourceNotes {
    pub(super) fn read(
        row: &Evidence,
        row_note: &Value,
        meta: &Evidence,
        meta_note: &Value,
    ) -> StoreResult<Self> {
        let mut raw = meta_note
            .get("raw_metadata_capture")
            .cloned()
            .ok_or_else(|| invalid("missing raw capture"))?;
        let raw_ref = text(&raw, "archive_manifest")?.to_owned();
        if text(&raw, "archive_phase")? != census_crawl::milesplit::OWNED_CAPTURE_PHASE {
            return Err(invalid("foreign raw metadata archive phase"));
        }
        let fields = raw
            .as_object_mut()
            .ok_or_else(|| invalid("raw capture object"))?;
        fields.remove("archive_manifest");
        fields.remove("archive_phase");
        let raw_capture = capture(raw)?;
        let owned_capture = capture(
            meta_note
                .get("owned_capture")
                .cloned()
                .ok_or_else(|| invalid("owned capture missing"))?,
        )?;
        let reference = ResultSetRef::parse(&raw_capture.url)
            .ok_or_else(|| invalid("raw result-set URL is invalid"))?;
        validate(
            row,
            row_note,
            meta,
            meta_note,
            &reference,
            &owned_capture,
            &raw_capture,
        )?;
        let meet = reference
            .meet_id
            .parse::<u64>()
            .map_err(|_| invalid("meet ID"))?;
        Ok(Self {
            owned: OwnedAcquisitionRef::from_capture(meet, &owned_capture)
                .map_err(|_| invalid("owned archive reference"))?,
            raw: RawMetadataArchiveRef::parse(&raw_ref)
                .map_err(|_| invalid("raw archive reference"))?,
            locator: text(row_note, "locator")?.into(),
            reference,
            owned_capture,
            raw_capture,
        })
    }
}

fn capture(value: Value) -> StoreResult<FetchOutcome> {
    let capture: Capture = serde_json::from_value(value).map_err(|source| StoreError::Decode {
        key: "participation capture provenance".into(),
        source,
    })?;
    if !capture.request_body.is_null()
        || !capture.request_headers.is_empty()
        || capture.method != "GET"
        || capture.status != 200
    {
        return Err(invalid("unsupported retained capture request"));
    }
    Ok(FetchOutcome {
        url: capture.url,
        response_url: capture.response_url,
        method: capture.method,
        status: capture.status,
        content_digest: capture.content_digest,
        bytes: capture.bytes,
        fetched_at: capture.fetched_at,
        content_type: capture.content_type,
        from_cache: false,
        body: Vec::new(),
    })
}

fn validate(
    row: &Evidence,
    row_note: &Value,
    meta: &Evidence,
    meta_note: &Value,
    reference: &ResultSetRef,
    owned: &FetchOutcome,
    raw: &FetchOutcome,
) -> StoreResult<()> {
    if row.method != EvidenceMethod::Parsed
        || meta.method != EvidenceMethod::Parsed
        || row.source.id != reference.site.source_id()
        || meta.source.id != reference.site.source_id()
        || row.source.url.as_deref() != Some(owned.url.as_str())
        || meta.source.url.as_deref() != Some(raw.url.as_str())
        || row.observed_on != owned.fetched_at
        || meta.observed_on != raw.fetched_at
        || text(row_note, "capture_url")? != owned.url
        || text(row_note, "sha256")? != owned.content_digest
        || text(row_note, "acquired_at")? != owned.fetched_at
        || text(row_note, "raw_metadata_url")? != raw.url
        || text(meta_note, "meet_id")? != reference.meet_id
        || text(meta_note, "result_set_id")? != reference.rsid
        || text(meta_note, "projection_revision")? != census_crawl::milesplit::RESULT_SET_PHASE
        || text(meta_note, "parser_revision")? != "milesplit_owned_v4_raw_metadata_v1"
    {
        return Err(invalid(
            "canonical source provenance changes its acquisition or result-set owner",
        ));
    }
    Ok(())
}

pub(super) fn text<'a>(value: &'a Value, field: &str) -> StoreResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("participation note omits a required text field"))
}

fn keys(value: &Value, allowed: &[&str]) -> bool {
    value.as_object().is_some_and(|fields| {
        fields.len() == allowed.len() && fields.keys().all(|key| allowed.contains(&key.as_str()))
    })
}
