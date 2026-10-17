#[path = "r14_owner/mod.rs"]
mod owner;

#[path = "r14_native_support/mod.rs"]
mod support;

#[path = "r14_native_probe/cases.rs"]
mod cases;

#[path = "r14_fixture_input/mod.rs"]
mod fixture_input;

#[path = "r14_fixture_input/schema.rs"]
mod fixture_schema;

use cases::{cancellation_case, capture_case};

use std::error::Error;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

use athleticnet_browser::request::{endpoint, safe, RequestAction, RequestSpec};
use athleticnet_browser::{BrowserManager, BrowserSettings, BrowserState};

type ProbeError = Box<dyn Error + Send + Sync>;
type ProbeResult<T> = Result<T, ProbeError>;

const CHROMIUM: &str = "/usr/lib/chromium/chromium";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const CHALLENGE_WAIT: Duration = Duration::from_secs(2);
const NORMAL_BODY: &[u8] = b"wire-smoke-ok";
const DATA_PATHS: [&str; 3] = ["/ok", "/decoded-over", "/wire-over"];

fn main() -> ProbeResult<()> {
    let Some(raw) = std::env::args_os().nth(1) else {
        return Err(fail("usage: r14_native_probe <fixtureDir>".to_string()));
    };
    let path = PathBuf::from(raw);
    let fixtures =
        fs::canonicalize(&path).map_err(|error| fail(format!("fixture dir: {error}")))?;
    if !fixtures.is_dir() {
        return Err(fail(format!("not a directory: {}", fixtures.display())));
    }
    let mut runtime = tokio::runtime::Builder::new_current_thread();
    runtime.enable_all();
    let runtime = runtime
        .build()
        .map_err(|error| fail(format!("runtime: {error}")))?;
    runtime.block_on(run(&fixtures))
}

fn fail(message: String) -> ProbeError {
    message.into()
}

fn err(name: &str, detail: String) -> ProbeError {
    fail(format!("{name}: {detail}"))
}

async fn run(fixtures: &Path) -> ProbeResult<()> {
    verify_fixture_bytes(fixtures)?;
    let mut failures: Vec<String> = Vec::new();
    for (name, path, exact_body) in [
        ("normal", "/ok", true),
        ("decoded-over", "/decoded-over", false),
        ("wire-over", "/wire-over", false),
    ] {
        if let Err(error) = capture_case(fixtures, name, path, exact_body).await {
            failures.push(error.to_string());
        }
    }
    println!("CAPTURE_CASES_COMPLETE");
    if let Err(error) = cancellation_case(fixtures).await {
        failures.push(error.to_string());
    }
    if failures.is_empty() {
        println!("R14_NATIVE_PROBE_ALL_CASES_OK");
        return Ok(());
    }
    Err(fail(format!(
        "{} case(s) failed: {}",
        failures.len(),
        failures.join(" | ")
    )))
}

fn verify_fixture_bytes(fixtures: &Path) -> ProbeResult<()> {
    let manifest = fixture_input::read_manifest(fixtures.join("manifest.json").as_path())
        .map_err(|error| fail(format!("manifest: {error}")))?;
    fixture_input::read_artifact(
        &fixtures.join("wire-over.gz"),
        &manifest.wire,
        fixture_input::ArtifactKind::Wire,
    )
    .map_err(|error| fail(format!("wire fixture: {error}")))?;
    println!("FIXTURE wire-over.gz bytes={}", manifest.wire.encoded);
    fixture_input::read_artifact(
        &fixtures.join("decoded-over.gz"),
        &manifest.decoded,
        fixture_input::ArtifactKind::Decoded,
    )
    .map_err(|error| fail(format!("decoded fixture: {error}")))?;
    println!("FIXTURE decoded-over.gz bytes={}", manifest.decoded.encoded);
    Ok(())
}

fn fresh_profile(fixtures: &Path, name: &str) -> ProbeResult<PathBuf> {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let root = std::env::var_os("R14_NATIVE_PROFILE_ROOT")
        .map_or_else(|| fixtures.to_path_buf(), PathBuf::from);
    let profile = root.join(format!(
        "profile-{name}-{}-{}",
        std::process::id(),
        stamp.as_nanos()
    ));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&profile)
        .map_err(|error| err(name, format!("profile create: {error}")))?;
    let metadata = fs::symlink_metadata(&profile)
        .map_err(|error| err(name, format!("profile metadata: {error}")))?;
    let mode = metadata.permissions().mode() & 0o777;
    if !metadata.is_dir() || mode != 0o700 {
        return Err(err(name, format!("profile type or mode {mode:o}")));
    }
    println!("PROFILE {name} mode={mode:o} {}", profile.display());
    Ok(profile)
}

fn start_server(fixtures: &Path, name: &str) -> ProbeResult<support::Server> {
    support::Server::start(fixtures).map_err(|error| err(name, format!("server start: {error}")))
}

fn loopback_origin(origin: &url::Url, name: &str) -> ProbeResult<url::Url> {
    let loopback = origin.scheme() == "http" && origin.host_str() == Some("127.0.0.1");
    if !loopback || origin.port().is_none() {
        return Err(err(name, format!("origin {origin} is not loopback http")));
    }
    Ok(origin.clone())
}

async fn launch(
    profile: &Path,
    origin: url::Url,
    name: &str,
) -> ProbeResult<(std::sync::Arc<BrowserManager>, owner::ProbeSupervisor)> {
    let settings = BrowserSettings {
        cdp_endpoint: None,
        executable: PathBuf::from(CHROMIUM),
        profile_dir: profile.to_path_buf(),
        source_origin: origin,
        tabs: 1,
        request_timeout: REQUEST_TIMEOUT,
        challenge_wait: CHALLENGE_WAIT,
        headed: false,
    };
    owner::launch(settings)
        .await
        .map_err(|error| err(name, format!("launch: {error}")))
}

fn ready_deviation(manager: &BrowserManager) -> Vec<String> {
    let state = manager.status().state;
    if state == BrowserState::Ready {
        Vec::new()
    } else {
        vec![format!("state {state:?} was not Ready")]
    }
}

fn spec_for(origin: &url::Url, path: &str, name: &str) -> ProbeResult<(RequestSpec, String)> {
    let url = endpoint(origin, path).map_err(|error| err(name, format!("endpoint: {error}")))?;
    let spec = match safe(url, RequestAction::Fetch {}) {
        Ok(spec) => spec,
        Err(error) => return Err(err(name, format!("spec: {error}"))),
    };
    let expected_url = spec.url.to_string();
    Ok((spec, expected_url))
}
