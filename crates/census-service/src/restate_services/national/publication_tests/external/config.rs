use super::super::TestResult;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) const OLD_COMMIT: &str = "d54339183d0c50511f8cbee354c2b03414bfd440";
const ISOLATION: &str = "separate-empty-scratch-stores-with-provider-and-model-egress-disabled";
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;

pub(super) struct Settings {
    pub old_url: String,
    pub current_url: String,
    pub evidence: PathBuf,
    pub old: Artifact,
    pub current: Artifact,
}

#[derive(Serialize)]
pub(super) struct Artifact {
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
    pub declared_commit: String,
}

impl Settings {
    pub(super) fn read(get: impl Fn(&str) -> Option<String>) -> TestResult<Self> {
        let required = |key: &str| -> TestResult<String> {
            get(key)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("external qualification requires {key}").into())
        };
        check!(eq; required("CENSUS_PUBLICATION_ISOLATION")?, ISOLATION);
        let old_url = endpoint(&required("CENSUS_PUBLICATION_OLD_ENDPOINT")?)?;
        let current_url = endpoint(&required("CENSUS_PUBLICATION_CURRENT_ENDPOINT")?)?;
        distinct(&old_url, &current_url)?;
        check!(eq; required("CENSUS_PUBLICATION_OLD_COMMIT")?, OLD_COMMIT);
        let current_commit = required("CENSUS_PUBLICATION_CURRENT_COMMIT")?;
        check!(
            current_commit.len() == 40
                && current_commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "candidate commit must be a full Git identifier"
        );
        let old = artifact(
            Path::new(&required("CENSUS_PUBLICATION_OLD_BINARY")?),
            OLD_COMMIT,
        )?;
        let current = artifact(
            Path::new(&required("CENSUS_PUBLICATION_CURRENT_BINARY")?),
            &current_commit,
        )?;
        check!(
            old.sha256 != current.sha256,
            "historical and candidate artifacts must differ"
        );
        let evidence = PathBuf::from(required("CENSUS_PUBLICATION_EVIDENCE")?);
        check!(
            evidence.is_absolute(),
            "evidence directory must be absolute"
        );
        match std::fs::symlink_metadata(&evidence) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(_) => return Err("evidence directory already exists".into()),
        }
        Ok(Self {
            old_url,
            current_url,
            evidence,
            old,
            current,
        })
    }
}

pub(super) fn endpoint(value: &str) -> TestResult<String> {
    let url = url::Url::parse(value)?;
    let loopback = matches!(url.host(), Some(url::Host::Ipv4(address)) if address == std::net::Ipv4Addr::LOCALHOST)
        || matches!(url.host(), Some(url::Host::Ipv6(address)) if address == std::net::Ipv6Addr::LOCALHOST);
    check!(
        url.scheme() == "http" && loopback,
        "external endpoint must be literal HTTP loopback"
    );
    check!(
        url.port().is_some_and(|port| port >= 20000),
        "external endpoint needs an isolated explicit high port"
    );
    check!(
        url.username().is_empty() && url.password().is_none(),
        "endpoint credentials refused"
    );
    check!(
        url.query().is_none() && url.fragment().is_none(),
        "endpoint query or fragment refused"
    );
    check!(url.path() == "/", "external endpoint must be a base URL");
    Ok(url.join("invoke/NationalCensus/run")?.to_string())
}

pub(super) fn distinct(old: &str, current: &str) -> TestResult {
    check!(
        old != current,
        "historical and candidate endpoints must differ"
    );
    Ok(())
}

pub(super) fn artifact(path: &Path, revision: &str) -> TestResult<Artifact> {
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    check!(
        metadata.is_file() && metadata.len() > 0 && metadata.len() <= MAX_ARTIFACT_BYTES,
        "artifact must be a nonempty regular file of at most 512 MiB"
    );
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(u64::try_from(read)?)
            .ok_or("artifact byte count overflow")?;
        check!(
            bytes <= MAX_ARTIFACT_BYTES,
            "artifact grew beyond its byte budget"
        );
        hasher.update(buffer.get(..read).ok_or("artifact read length")?);
    }
    check!(eq; bytes, metadata.len());
    Ok(Artifact {
        path: path.canonicalize()?,
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
        declared_commit: revision.to_owned(),
    })
}
