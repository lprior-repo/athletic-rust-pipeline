#[path = "r14_owner/mod.rs"]
mod owner;

use std::error::Error;
use std::fs::DirBuilder;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::time::Duration;

use athleticnet_browser::request::{endpoint, safe, RequestAction, RequestSpec};
use athleticnet_browser::{BrowserOutcome, BrowserSettings, BrowserState};

type ProbeResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

fn main() -> ProbeResult {
    let mut arguments = std::env::args().skip(1);
    let origin = arguments.next().ok_or("missing loopback origin")?;
    let profile = PathBuf::from(arguments.next().ok_or("missing fresh profile path")?);
    let case = arguments
        .next()
        .ok_or("missing case: duplicate or redirect")?;
    if arguments.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let origin = url::Url::parse(&origin)?;
    if origin.scheme() != "http"
        || origin.host_str() != Some("127.0.0.1")
        || origin.port().is_none()
    {
        return Err("fixture origin must be a dedicated loopback port".into());
    }
    let path = match case.as_str() {
        "duplicate" => "/target",
        "redirect" => "/redirect-target",
        _ => return Err("unsupported probe case".into()),
    };
    let spec = safe(endpoint(&origin, path)?, RequestAction::Fetch {})?;
    let second_spec = safe(endpoint(&origin, path)?, RequestAction::Fetch {})?;
    DirBuilder::new().mode(0o700).create(&profile)?;
    let settings = BrowserSettings {
        cdp_endpoint: None,
        executable: PathBuf::from("/usr/bin/chromium"),
        profile_dir: profile,
        source_origin: origin,
        tabs: 1,
        request_timeout: Duration::from_secs(30),
        challenge_wait: Duration::from_secs(2),
        headed: false,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(probe(settings, case, spec, second_spec))
}

async fn probe(
    settings: BrowserSettings,
    case: String,
    spec: RequestSpec,
    second_spec: RequestSpec,
) -> ProbeResult {
    let expected_url = spec.url.to_string();
    let (manager, mut supervisor) = owner::launch(settings).await?;
    let ready = manager.status().state;
    let outcome = manager.fetch(spec).await;
    let repeat = if case == "duplicate" {
        Some(manager.fetch(second_spec).await)
    } else {
        None
    };
    let (report, failure) = manager.shutdown().await;
    supervisor.stop_and_join()?;
    let state = manager.status().state;
    let alive = manager.is_alive();
    println!("CASE={case} READY={ready:?} OUTCOME={outcome:?}");
    println!("CASE={case} REPEAT={repeat:?}");
    println!("CASE={case} STATE={state:?} ALIVE={alive} DRAIN={report:?} FAILURE={failure:?}");
    let terminal = report
        .completed
        .checked_add(report.cancelled)
        .and_then(|count| count.checked_add(report.aborted))
        .and_then(|count| count.checked_add(report.panicked));
    if ready != BrowserState::Ready
        || state != BrowserState::Stopped
        || alive
        || failure.is_some()
        || report.remaining != 0
        || report.aborted != 0
        || report.panicked != 0
        || terminal != Some(report.accepted)
    {
        return Err("native ownership or drain certificate failed".into());
    }
    for outcome in std::iter::once(outcome).chain(repeat) {
        verify_outcome(&case, &expected_url, outcome)?;
    }
    Ok(())
}

fn verify_outcome(case: &str, expected_url: &str, outcome: BrowserOutcome) -> ProbeResult {
    match (case, outcome) {
        ("duplicate", BrowserOutcome::Captured(capture)) => {
            if capture.response.status.as_u16() != 200
                || capture.response.body.as_slice() != b"OWNED:session-cookie-request"
                || capture.response.response_url.as_deref() != Some(expected_url)
            {
                return Err("captured a body other than the actual owned session request".into());
            }
        }
        ("redirect", BrowserOutcome::Captured(_)) => {
            return Err("published a successful body from a redirecting request".into());
        }
        ("duplicate", BrowserOutcome::Failed(failure)) => {
            return Err(format!("legitimate owned request refused: {:?}", failure.error).into());
        }
        ("redirect", BrowserOutcome::Failed(failure)) => {
            println!("EXPLICIT_REFUSAL={:?}", failure.error);
        }
        _ => return Err("unexpected probe result".into()),
    }
    Ok(())
}
