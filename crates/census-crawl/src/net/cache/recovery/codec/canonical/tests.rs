use super::Metadata;
use crate::net::cache::{content_digest, CacheMeta, Capture, EligibleCacheV2};
use crate::net::{RedirectHop, RepresentationHeaders};
use census_domain::model::*;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const FIRST: &str = "https://mi.milesplit.com/athletes/13749840-old-slug";
const FINAL: &str = "https://mi.milesplit.com/athletes/13749840-new-slug";
const OLD: &str = r#"{
  "bytes": 4,
  "content_digest": "230d8358dc8e8890b4c58deeb62912ee2f20357ae92a5cc861b98e68fe31acb5",
  "fetched_at": "2026-10-21T08:00:02Z",
  "method": "GET",
  "status": 200,
  "url": "https://mi.milesplit.com/athletes/13749840-old-slug"
}"#;

fn dispatch(hop: u8) -> TestResult<AcquisitionDispatchId> {
    Ok(AcquisitionDispatchId::new(
        AcquisitionAttemptId::new(
            StoreIdentity::parse(&"A".repeat(64))?,
            AcquisitionWorkId::parse(&"b".repeat(64))?,
            AttemptOrdinal::new(2)?,
        ),
        HopOrdinal::new(hop)?,
    ))
}

fn hop(ordinal: u8) -> TestResult<ObservedHttpHopV1> {
    let (url, start, received, status, protocol, headers) = if ordinal == 0 {
        (
            FIRST,
            "2026-10-21T08:00:00Z",
            "2026-10-21T08:00:01Z",
            302,
            ObservedHttpProtocolV1::Http2,
            vec![ObservedHttpHeaderV1::new(
                "location",
                FINAL.as_bytes().to_vec(),
            )?],
        )
    } else {
        (
            FINAL,
            "2026-10-21T08:00:01Z",
            "2026-10-21T08:00:02Z",
            200,
            ObservedHttpProtocolV1::Http11,
            vec![
                ObservedHttpHeaderV1::new("x-evidence", vec![128, 255, b'\t'])?,
                ObservedHttpHeaderV1::new("content-type", b"text/html".to_vec())?,
                ObservedHttpHeaderV1::new("x-evidence", b"second".to_vec())?,
            ],
        )
    };
    Ok(ObservedHttpHopV1::new(
        dispatch(ordinal)?,
        ObservedHttpRequestV1::new(
            url,
            start,
            vec![
                ObservedHttpHeaderV1::new("user-agent", b"Census/1.0".to_vec())?,
                ObservedHttpHeaderV1::new("accept-encoding", b"identity".to_vec())?,
            ],
        )?,
        ObservedHttpResponseV1::new(protocol, status, url, received, headers)?,
    )?)
}

fn metadata() -> TestResult<CacheMeta> {
    let digest = content_digest(b"body");
    Ok(CacheMeta {
        url: FIRST.into(),
        response_url: Some(FINAL.into()),
        method: "GET".into(),
        representation: RepresentationHeaders::from_entries(vec![
            ("accept-encoding".into(), "identity".into()),
            ("user-agent".into(), "Census/1.0".into()),
        ])?,
        status: 200,
        content_digest: digest.clone(),
        bytes: 4,
        fetched_at: "2026-10-21T08:00:02Z".into(),
        etag: Some("\"capture-v1\"".into()),
        last_modified: Some("Wed, 21 Oct 2026 08:00:00 GMT".into()),
        content_type: Some("text/html".into()),
        redirects: vec![RedirectHop {
            status: 302,
            url: FINAL.into(),
        }],
        observed_http: Some(NormalizedObservedHttpV1::new(
            vec![hop(0)?, hop(1)?],
            "2026-10-21T08:00:03Z",
            AcquisitionDigest::parse(&digest)?,
            4,
        )?),
    })
}

#[test]
fn absent_observations_retain_the_independent_original_literal_bytes() -> TestResult {
    let metadata: CacheMeta = serde_json::from_str(OLD)?;
    check!(metadata.observed_http.is_none());
    let bytes = serde_json::to_vec_pretty(&Metadata(&metadata))?;
    check!(eq; bytes, OLD.as_bytes());
    check!(eq; Capture::from_meta(&metadata, Path::new("metadata.json"))?.encoded, bytes);
    check!(eq; serde_json::from_slice::<CacheMeta>(&bytes)?, metadata);
    Ok(())
}

#[test]
fn observed_stream_matches_actual_capture_bytes_at_every_nested_object() -> TestResult {
    let metadata = metadata()?;
    let capture = Capture::from_meta(&metadata, Path::new("metadata.json"))?;
    check!(ne; serde_json::to_vec_pretty(&metadata)?, capture.encoded);
    let streamed = serde_json::to_vec_pretty(&Metadata(&metadata))?;
    check!(eq; streamed, capture.encoded);
    let reopened: CacheMeta = serde_json::from_slice(&streamed)?;
    check!(eq; reopened, metadata);
    let observed = reopened.observed_http.ok_or("original observations")?;
    let final_hop = observed.hops().get(1).ok_or("missing final observed hop")?;
    let headers = final_hop.response().headers();
    check!(eq; headers.first().ok_or("missing first final-response header")?.value(), &[128, 255, b'\t']);
    check!(eq; headers.get(1).ok_or("missing second final-response header")?.name(), "content-type");
    check!(eq; headers.get(2).ok_or("missing third final-response header")?.value(), b"second");
    let first_hop = observed
        .hops()
        .first()
        .ok_or("missing first observed hop")?;
    check!(eq; first_hop.dispatch().attempt().ordinal().get(), 2);
    Ok(())
}

#[test]
fn prior_eligible_envelope_keeps_original_observed_metadata_when_canonicalized() -> TestResult {
    let metadata = metadata()?;
    let capture = Capture::from_meta(&metadata, Path::new("metadata.json"))?;
    let occurrence = AcquisitionOccurrenceRefV1::new(
        dispatch(0)?.attempt().clone(),
        AcquisitionDigest::parse(&"d".repeat(64))?,
    );
    let envelope = EligibleCacheV2::new(metadata.clone(), occurrence.clone());
    let bytes = envelope.encode(Path::new("eligible.meta.json"))?;
    let previous: EligibleCacheV2 = serde_json::from_slice(&bytes)?;
    check!(eq; previous.original_occurrence, occurrence);
    check!(eq; previous.metadata, metadata);
    let preserved = serde_json::to_vec_pretty(&Metadata(&previous.metadata))?;
    check!(eq; preserved, capture.encoded);
    let reopened: CacheMeta = serde_json::from_slice(&preserved)?;
    check!(eq; reopened.observed_http, metadata.observed_http);
    check!(eq; reopened, metadata);
    Ok(())
}

#[test]
fn populated_original_fields_keep_capture_bytes_when_observations_are_absent() -> TestResult {
    let mut metadata = metadata()?;
    metadata.observed_http = None;
    let captured = Capture::from_meta(&metadata, Path::new("metadata.json"))?;
    let streamed = serde_json::to_vec_pretty(&Metadata(&metadata))?;
    check!(eq; streamed, captured.encoded);
    check!(eq; serde_json::from_slice::<CacheMeta>(&streamed)?, metadata);
    Ok(())
}
