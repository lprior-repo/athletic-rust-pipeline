use super::{io, Result};
use census_crawl::milesplit::{
    parse_owned_meet, parse_raw, OwnedMeetPage, OwnedMeetVerdict, ResultSetRef, OWNED_FIELDS,
};
use census_domain::model::Sport;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::path::{Path, PathBuf};

pub(super) const RAW_URL: &str =
    "https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266814/raw";
const API_SHA: &str = "8db9804ebc2d36b6f7ec1e2a289b2b8ab28610e0063f0ee9f54245eb128071ed";

pub(super) struct Arguments {
    pub root: PathBuf,
    pub paths: [(PathBuf, PathBuf); 4],
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
    pub key: String,
}

pub(super) struct Inputs {
    pub captures: [Capture; 4],
    pub owned: OwnedMeetPage,
    pub facts: Value,
}

struct Specification {
    url: String,
    sha: &'static str,
    size: usize,
    acquired_at: &'static str,
    key: &'static str,
}

pub(super) fn arguments() -> Result<Arguments> {
    let args: Vec<_> = std::env::args_os().skip(1).take(10).collect();
    let [root, api, api_meta, raw, raw_meta, index, index_meta, roster, roster_meta] =
        args.as_slice()
    else {
        return Err("usage: qualification_bound_projection FRESH_ROOT API_BODY API_META_JSON RAW_BODY RAW_META_JSON INDEX_BODY INDEX_META_JSON ROSTER_BODY ROSTER_META_JSON".into());
    };
    Ok(Arguments {
        root: root.into(),
        paths: [
            (api.into(), api_meta.into()),
            (raw.into(), raw_meta.into()),
            (index.into(), index_meta.into()),
            (roster.into(), roster_meta.into()),
        ],
    })
}

fn specifications() -> [Specification; 4] {
    [
        Specification {
            url: format!("https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={OWNED_FIELDS}"),
            sha: API_SHA, size: 311_763, acquired_at: "2026-10-01T23:44:16Z",
            key: "52e0b5d61b6c7de90be2a35dd5fc3f42",
        },
        Specification {
            url: RAW_URL.into(), sha: "729850b479e5782aa3e4ade7740cd46b8ffd8f35db79a873abd4ed0b3fb0f6cf",
            size: 80_740, acquired_at: "2026-09-28T10:11:24Z", key: "c7791d114036f1a83166fcb81998032d",
        },
        Specification {
            url: "https://al.milesplit.com/teams".into(),
            sha: "3592a6bc554fd0c2592e69886f77cebe1a22cda3d74edc353e6f2f2b20e6f346",
            size: 193_420, acquired_at: "2026-09-27T23:45:18Z", key: "a361c173de86509e4dbe70cc8212cea8",
        },
        Specification {
            url: "https://al.milesplit.com/teams/38332-abbeville-high-school/roster".into(),
            sha: "169170d338d54baef21587776f6d3a7eff2f8103c6467a14171773bf8f071730",
            size: 203_886, acquired_at: "2026-09-27T23:45:20Z", key: "b0baf6da6f1b721f0118c6d070a9ca2d",
        },
    ]
}

pub(super) fn load(args: &Arguments) -> Result<Inputs> {
    let captures = args
        .paths
        .iter()
        .zip(specifications())
        .map(|((body, meta), spec)| read_capture(body, meta, &spec))
        .collect::<Result<Vec<_>>>()?;
    let captures: [Capture; 4] = captures.try_into().map_err(|_| "four captures required")?;
    let [api, raw, index, roster] = &captures;
    let OwnedMeetVerdict::Parsed(owned) = parse_owned_meet(&api.body, 725218) else {
        return Err("authentic structured capture did not parse".into());
    };
    if owned.published_rows > io::MAX_ROWS || owned.rows.is_empty() {
        return Err("structured rows absent or exceed smoke bound".into());
    }
    let reference = ResultSetRef::parse(&raw.metadata.url).ok_or("raw URL not admitted")?;
    let page = parse_raw(std::str::from_utf8(&raw.body)?, &raw.metadata.url)?;
    if reference.url != raw.metadata.url
        || reference.site.jurisdiction() != UsJurisdiction::Alabama
        || page.school_year.get() != 2025
        || page.sport != Some(Sport::OutdoorTrack)
        || page.meet.date != "2026-03-27"
    {
        return Err("authentic meet/season/URL prerequisite differs".into());
    }
    let facts = json!({
        "captures": captures.iter().zip(&args.paths).map(|(capture, (body, meta))| json!({
            "metadata": capture.metadata, "cache_key": capture.key,
            "original_body_path": body, "original_metadata_path": meta,
            "metadata_sha256": io::digest(&capture.metadata_bytes)
        })).collect::<Vec<_>>(),
        "published_rows": owned.published_rows, "parsed_individual_rows": owned.rows.len(),
        "requested_individual_rows": owned.rows.iter().filter(|row| row.result_set_id == 1266814).count(),
        "source_completeness": owned.completeness, "ownership_complete": owned.ownership_complete(),
        "individual_parse_complete": owned.individual_parse_complete(), "rejections": owned.rejected,
        "meet_date": page.meet.date, "meet_school_year": page.school_year, "meet_sport": page.sport,
        "roster_capture_date": roster.metadata.fetched_at, "index_capture_date": index.metadata.fetched_at,
        "input_role": "historical authentic captures; qualified roster school only, no roster population intake"
    });
    Ok(Inputs {
        captures,
        owned,
        facts,
    })
}

fn read_capture(body_path: &Path, meta_path: &Path, spec: &Specification) -> Result<Capture> {
    let body = io::read(body_path, io::MAX_BODY)?;
    let metadata_bytes = io::read(meta_path, io::MAX_META)?;
    let metadata: Metadata = serde_json::from_slice(&metadata_bytes)?;
    let key = cache_key(&metadata.url)?;
    if body.len() != spec.size
        || io::digest(&body) != spec.sha
        || metadata.url != spec.url
        || metadata.method != "GET"
        || metadata.status != 200
        || metadata.content_digest != spec.sha
        || metadata.bytes != body.len()
        || metadata.fetched_at != spec.acquired_at
        || key != spec.key
    {
        return Err(format!(
            "{} / {} contradict original capture contract",
            body_path.display(),
            meta_path.display()
        )
        .into());
    }
    chrono::DateTime::parse_from_rfc3339(&metadata.fetched_at)?;
    Ok(Capture {
        body,
        metadata_bytes,
        metadata,
        key,
    })
}

fn cache_key(url: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let digest = hasher.finalize();
    let mut key = String::new();
    key.try_reserve_exact(32)?;
    digest
        .get(..16)
        .ok_or("SHA256 key prefix absent")?
        .iter()
        .try_for_each(|byte| write!(&mut key, "{byte:02x}"))?;
    Ok(key)
}

pub(super) fn unchanged(args: &Arguments, inputs: &Inputs) -> Result<bool> {
    args.paths
        .iter()
        .zip(&inputs.captures)
        .try_fold(true, |same, ((body, meta), capture)| {
            let body_same = io::read(body, io::MAX_BODY)? == capture.body;
            let meta_same = io::read(meta, io::MAX_META)? == capture.metadata_bytes;
            Ok(same && body_same && meta_same)
        })
}
