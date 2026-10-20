use super::{charge, frozen::assets, invalid, MAX_ARCHIVE_BYTES};
use census_crawl::net::FetchOutcome;
use census_domain::roster_affiliation_authority::RosterMemberObservationV1;
use census_store::{StoreError, StoreResult};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

mod file;

#[derive(Deserialize)]
struct Metadata {
    url: String,
    #[serde(default)]
    response_url: Option<String>,
    method: String,
    status: u16,
    content_digest: String,
    bytes: usize,
    fetched_at: String,
    content_type: Option<String>,
}

enum Source {
    Live(PathBuf),
    Frozen(assets::Source),
}

pub(super) struct Archive {
    source: Source,
    bytes: u64,
    retention: Option<assets::Sink>,
}

impl Archive {
    pub(super) fn new(store_root: &Path) -> StoreResult<Self> {
        let root = store_root.join("http/archive");
        for path in [
            store_root.join("http"),
            root.clone(),
            root.join("bodies"),
            root.join("captures"),
        ] {
            file::directory(&path)?;
        }
        Ok(Self {
            source: Source::Live(root),
            bytes: 0,
            retention: None,
        })
    }

    pub(super) fn capturing(store_root: &Path, sink: assets::Sink) -> StoreResult<Self> {
        let mut archive = Self::new(store_root)?;
        archive.retention = Some(sink);
        Ok(archive)
    }

    pub(super) fn frozen(source: assets::Source) -> Self {
        Self {
            source: Source::Frozen(source),
            bytes: 0,
            retention: None,
        }
    }

    pub(super) fn retention(self) -> Option<assets::Sink> {
        self.retention
    }

    pub(super) fn capture(
        &mut self,
        note: &RosterMemberObservationV1,
    ) -> StoreResult<Option<(FetchOutcome, String)>> {
        let path = format!("bodies/{}.body", note.capture_sha256());
        let Some(body) = self.read(&path, note.capture_bytes())? else {
            return Ok(None);
        };
        if body.len() != note.capture_bytes() || hash(&body) != note.capture_sha256() {
            return Err(invalid(
                "retained roster capture body differs from the pinned row",
            ));
        }
        let Some((meta, metadata_ref, original_metadata)) = self.metadata(note)? else {
            return Ok(None);
        };
        if let Some(sink) = &mut self.retention {
            sink.retain(
                note.capture_sha256(),
                &body,
                &metadata_ref,
                &original_metadata,
            )?;
        }
        let capture = FetchOutcome {
            url: meta.url,
            response_url: meta.response_url,
            method: meta.method,
            status: meta.status,
            content_digest: meta.content_digest,
            bytes: meta.bytes,
            fetched_at: meta.fetched_at,
            from_cache: true,
            content_type: meta.content_type,
            body,
        };
        Ok(Some((capture, metadata_ref)))
    }

    fn metadata(
        &mut self,
        note: &RosterMemberObservationV1,
    ) -> StoreResult<Option<(Metadata, String, Vec<u8>)>> {
        let mut found = None;
        for path in self.metadata_paths(note.capture_sha256())? {
            let name = path
                .rsplit('/')
                .next()
                .ok_or_else(|| invalid("archive metadata has no file name"))?;
            let digest = name
                .strip_suffix(".meta.json")
                .filter(|value| super::note::digest(value))
                .ok_or_else(|| invalid("archive metadata name is not a content digest"))?;
            let bytes = self
                .read(&path, 64 * 1024)?
                .ok_or_else(|| invalid("archive metadata disappeared during pinned read"))?;
            if hash(&bytes) != digest {
                return Err(invalid(
                    "archive metadata bytes do not match their immutable name",
                ));
            }
            let meta: Metadata =
                serde_json::from_slice(&bytes).map_err(|source| StoreError::Decode {
                    key: path.clone(),
                    source,
                })?;
            let reference = format!("archive/{path}");
            if matches(&meta, note)
                && found
                    .as_ref()
                    .is_none_or(|(_, previous, _): &(Metadata, String, Vec<u8>)| {
                        reference < *previous
                    })
            {
                found = Some((meta, reference, bytes));
            }
        }
        Ok(found)
    }

    fn metadata_paths(&self, digest: &str) -> StoreResult<Vec<String>> {
        let root = match &self.source {
            Source::Frozen(source) => return source.metadata(digest),
            Source::Live(root) => root,
        };
        let path = root.join("captures").join(digest);
        if !file::directory(&path)? {
            return Ok(Vec::new());
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&path).map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })? {
            if paths.len() >= 128 {
                return Err(invalid(
                    "retained roster capture metadata inventory exceeds 128 objects",
                ));
            }
            let entry = entry.map_err(|source| StoreError::Io {
                path: path.clone(),
                source,
            })?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| invalid("archive metadata name is not UTF-8"))?;
            paths.push(format!("captures/{digest}/{name}"));
        }
        paths.sort();
        Ok(paths)
    }

    fn read(&mut self, path: &str, maximum: usize) -> StoreResult<Option<Vec<u8>>> {
        match &self.source {
            Source::Live(root) => file::read(&root.join(path), maximum, &mut self.bytes),
            Source::Frozen(source) => source.read(path, maximum, &mut self.bytes),
        }
    }
}

fn matches(meta: &Metadata, note: &RosterMemberObservationV1) -> bool {
    meta.url == note.capture_url()
        && meta.response_url.as_deref() == note.response_url()
        && meta.method == "GET"
        && meta.status == 200
        && meta.content_digest == note.capture_sha256()
        && meta.bytes == note.capture_bytes()
        && meta.fetched_at == note.fetched_at()
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
