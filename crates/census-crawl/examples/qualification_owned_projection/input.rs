use super::{io, Result};
use census_crawl::milesplit::{
    parse_owned_meet, parse_raw, OwnedCompleteness, OwnedMeetVerdict, OwnedRejectionKind,
    ResultSetRef, OWNED_FIELDS,
};
use census_domain::model::{CanonicalSchool, SourceNamespace, Sport};
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::PathBuf;

pub(super) const API_SHA: &str = "8db9804ebc2d36b6f7ec1e2a289b2b8ab28610e0063f0ee9f54245eb128071ed";
pub(super) const RAW_SHA: &str = "729850b479e5782aa3e4ade7740cd46b8ffd8f35db79a873abd4ed0b3fb0f6cf";
pub(super) const RAW_URL: &str =
    "https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266814/raw";

pub(super) struct Arguments {
    pub root: PathBuf,
    pub api_body: PathBuf,
    pub api_meta: PathBuf,
    pub raw_body: PathBuf,
    pub raw_meta: PathBuf,
    pub schools: PathBuf,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Metadata {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub content_digest: String,
    pub bytes: usize,
    pub fetched_at: String,
}

pub(super) struct Capture {
    pub body: Vec<u8>,
    pub metadata_bytes: Vec<u8>,
    pub metadata: Metadata,
}

pub(super) struct Inputs {
    pub api: Capture,
    pub raw: Capture,
    pub schools: Vec<u8>,
    pub facts: Value,
}

pub(super) fn arguments() -> Result<Arguments> {
    let args: Vec<_> = std::env::args_os().skip(1).take(8).collect();
    let [root, api_body, api_meta, raw_body, raw_meta, schools] = args.as_slice() else {
        return Err("usage: qualification_owned_projection FRESH_ROOT API_BODY API_META_JSON RAW_BODY RAW_META_JSON CANONICAL_SCHOOLS_JSONL".into());
    };
    Ok(Arguments {
        root: root.into(),
        api_body: api_body.into(),
        api_meta: api_meta.into(),
        raw_body: raw_body.into(),
        raw_meta: raw_meta.into(),
        schools: schools.into(),
    })
}

pub(super) fn load(args: &Arguments) -> Result<Inputs> {
    let api_url = format!("https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={OWNED_FIELDS}");
    let api = read_capture(
        &args.api_body,
        &args.api_meta,
        &api_url,
        API_SHA,
        311_763,
        "2026-10-01T23:44:16Z",
    )?;
    let raw = read_capture(
        &args.raw_body,
        &args.raw_meta,
        RAW_URL,
        RAW_SHA,
        80_740,
        "2026-09-28T10:11:24Z",
    )?;
    let reference =
        ResultSetRef::parse(&raw.metadata.url).ok_or("original raw URL not admitted")?;
    if reference.url != raw.metadata.url
        || reference.meet_id != "725218"
        || reference.rsid != "1266814"
        || reference.site.jurisdiction() != UsJurisdiction::Alabama
    {
        return Err("production reference mutates or mismatches original raw URL".into());
    }
    let schools = io::read(&args.schools, io::MAX_BODY)?;
    let facts = json!({
        "owned_capture": owned_facts(&api.body)?,
        "raw_metadata": raw_facts(&raw.body)?,
        "canonical_school_input": school_facts(&schools)?,
        "api": api.metadata, "raw": raw.metadata,
        "api_original_metadata_sha256": io::digest(&api.metadata_bytes),
        "raw_original_metadata_sha256": io::digest(&raw.metadata_bytes),
        "school_input_sha256": io::digest(&schools),
        "api_cache_key": cache_key(&api.metadata.url)?,
        "raw_cache_key": cache_key(&raw.metadata.url)?,
    });
    Ok(Inputs {
        api,
        raw,
        schools,
        facts,
    })
}

fn read_capture(
    body_path: &std::path::Path,
    meta_path: &std::path::Path,
    url: &str,
    sha: &str,
    size: usize,
    acquired_at: &str,
) -> Result<Capture> {
    let body = io::read(body_path, io::MAX_BODY)?;
    if body.len() != size || io::digest(&body) != sha {
        return Err(format!(
            "{} differs from complete original capture",
            body_path.display()
        )
        .into());
    }
    let metadata_bytes = io::read(meta_path, io::MAX_META)?;
    let metadata: Metadata = serde_json::from_slice(&metadata_bytes)?;
    if metadata.url != url
        || metadata.method != "GET"
        || metadata.status != 200
        || metadata.content_digest != sha
        || metadata.bytes != body.len()
        || metadata.fetched_at != acquired_at
    {
        return Err(format!(
            "{} contradicts original capture metadata or production URL contract",
            meta_path.display()
        )
        .into());
    }
    chrono::DateTime::parse_from_rfc3339(&metadata.fetched_at)?;
    Ok(Capture {
        body,
        metadata_bytes,
        metadata,
    })
}

fn owned_facts(bytes: &[u8]) -> Result<Value> {
    let verdict = parse_owned_meet(bytes, 725218);
    let OwnedMeetVerdict::Parsed(page) = verdict else {
        return Err(format!("complete original API capture not parsed: {verdict:?}").into());
    };
    let relays = page
        .rejected
        .iter()
        .filter(|r| r.kind == OwnedRejectionKind::TeamRelay)
        .count();
    if page.published_rows != 602
        || page.rows.len() != 560
        || relays != 42
        || page.rejected.len() != relays
        || page.ownership_complete()
        || page.completeness != OwnedCompleteness::Unknown
        || !page.individual_parse_complete()
    {
        return Err(
            "original capture domain interpretation differs from qualification input contract"
                .into(),
        );
    }
    Ok(
        json!({"published_rows": page.published_rows, "individuals": page.rows.len(),
        "recognized_team_relays": relays, "malformed_individuals": page.rejected.len().checked_sub(relays),
        "completeness": page.completeness, "ownership_complete": page.ownership_complete(),
        "individual_parse_complete": page.individual_parse_complete(),
        "requested_result_set_individuals": page.rows.iter().filter(|r| r.result_set_id == 1266814).count()}),
    )
}

fn raw_facts(bytes: &[u8]) -> Result<Value> {
    let page = parse_raw(std::str::from_utf8(bytes)?, RAW_URL)?;
    if page.meet.date != "2026-03-27"
        || page.sport != Some(Sport::OutdoorTrack)
        || page.school_year.get() != 2025
    {
        return Err("original raw meet date/sport/school year differs".into());
    }
    Ok(
        json!({"meet_name": page.meet.name, "meet_date": page.meet.date,
        "sport": page.sport, "school_year": page.school_year,
        "raw_rows_role": "metadata only; no name or grade fallback"}),
    )
}

fn school_facts(bytes: &[u8]) -> Result<Value> {
    let schools: Vec<CanonicalSchool> = std::str::from_utf8(bytes)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(io::MAX_ROWS.saturating_add(1))
        .map(serde_json::from_str)
        .collect::<std::result::Result<_, _>>()?;
    if schools.is_empty() || schools.len() > io::MAX_ROWS {
        return Err("canonical school input must be nonempty and bounded".into());
    }
    schools.iter().try_for_each(|school| -> Result<()> {
        if school.state != Some(UsJurisdiction::NorthCarolina)
            || school
                .source_identities
                .iter()
                .any(|id| id.namespace == SourceNamespace::MilesplitSchool)
            || !school.source_identities.iter().any(|id| {
                matches!(id.namespace, SourceNamespace::AssociationSchool { .. })
                    && !id.id.is_empty()
            })
        {
            return Err(
                "require truthful NC AssociationSchool input without ANY MilesplitSchool binding"
                    .into(),
            );
        }
        Ok(())
    })?;
    Ok(
        json!({"rows": schools.len(), "milesplit_school_bindings": 0,
        "owned_team_binding_absent": true, "school_identity_or_state_invented": false}),
    )
}

pub(super) fn cache_key(url: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let digest = hasher.finalize();
    let head = digest.get(..16).ok_or("SHA256 key prefix unavailable")?;
    let mut key = String::new();
    key.try_reserve_exact(32)?;
    head.iter()
        .try_for_each(|byte| write!(&mut key, "{byte:02x}"))?;
    Ok(key)
}

pub(super) fn unchanged(args: &Arguments, inputs: &Inputs) -> Result<bool> {
    let comparisons = [
        (&args.api_body, &inputs.api.body, io::MAX_BODY),
        (&args.api_meta, &inputs.api.metadata_bytes, io::MAX_META),
        (&args.raw_body, &inputs.raw.body, io::MAX_BODY),
        (&args.raw_meta, &inputs.raw.metadata_bytes, io::MAX_META),
        (&args.schools, &inputs.schools, io::MAX_BODY),
    ];
    comparisons
        .into_iter()
        .try_fold(true, |same, (path, expected, limit)| {
            let actual = io::read(path, limit)?;
            Ok(same && actual == *expected)
        })
}
