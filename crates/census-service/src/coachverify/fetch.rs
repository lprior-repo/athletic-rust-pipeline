use census_crawl::net::{FetchError, FetchOptions, Fetcher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Everything the gate needs beyond the fetcher itself.
#[derive(Debug, Clone)]
pub struct GateOptions {
    /// Do the XHR + Referer pass for rows that are not yet corroborated.
    pub xhr_pass: bool,
    /// Do the NSAA form POST for rows citing its export screen.
    pub nsaa_post: bool,
    /// `pdftotext` binary to inflate PDF citations with; `None` disables inflation.
    pub pdftotext: Option<String>,
    /// Ignore cached bodies and hit the network.
    pub refresh: bool,
}

impl Default for GateOptions {
    fn default() -> Self {
        Self {
            xhr_pass: true,
            nsaa_post: true,
            pdftotext: Some("pdftotext".to_string()),
            refresh: false,
        }
    }
}

/// How a single URL fetch ended.
enum Fetched {
    Text(String),
    Robots,
    Failed,
}

/// Fetch one citation and turn its body into text (PDFs are inflated).
async fn fetch_text(fetcher: &Fetcher, url: &str, xhr: bool, options: &GateOptions) -> Fetched {
    let headers = if xhr {
        vec![
            ("X-Requested-With".to_string(), "XMLHttpRequest".to_string()),
            ("Referer".to_string(), url.to_string()),
        ]
    } else {
        Vec::new()
    };
    let fetch_options = FetchOptions {
        refresh: options.refresh,
        allow_not_found: true,
        headers,
    };
    match fetcher.get(url, &fetch_options).await {
        Ok(outcome) => Fetched::Text(body_text(&outcome.body, options.pdftotext.as_deref())),
        Err(FetchError::Robots(_)) => Fetched::Robots,
        Err(_) => Fetched::Failed,
    }
}

/// POST the NSAA export screen for one row and turn the answer into text.
async fn fetch_nsaa_post(
    fetcher: &Fetcher,
    url: &str,
    school: &str,
    options: &GateOptions,
) -> Fetched {
    let form = vec![
        ("session".to_string(), String::new()),
        (
            "school".to_string(),
            crate::coachverify::nsaa_school(school),
        ),
    ];
    let fetch_options = FetchOptions {
        refresh: options.refresh,
        allow_not_found: true,
        headers: vec![("Referer".to_string(), url.to_string())],
    };
    match fetcher.post_form(url, &form, &fetch_options).await {
        Ok(outcome) => Fetched::Text(body_text(&outcome.body, options.pdftotext.as_deref())),
        Err(FetchError::Robots(_)) => Fetched::Robots,
        Err(_) => Fetched::Failed,
    }
}

/// A fetched body as text: `%PDF` payloads are inflated through `pdftotext` when available, and
/// anything else is decoded lossily — a page that is not valid UTF-8 must still be matchable.
pub fn body_text(body: &[u8], pdftotext: Option<&str>) -> String {
    if body.starts_with(b"%PDF") {
        if let Some(binary) = pdftotext {
            if let Some(text) = inflate_pdf(binary, body) {
                return text;
            }
        }
    }
    String::from_utf8_lossy(body).into_owned()
}

/// Counter for scratch file names: process id plus this counter is unique within a run.
static SCRATCH: AtomicU64 = AtomicU64::new(0);

/// Write the PDF to a scratch file, run `pdftotext` on it and read the text back. Every failure
/// (missing binary, unreadable output, unwritable scratch) returns `None` so the caller can fall
/// back to the raw bytes.
fn inflate_pdf(binary: &str, body: &[u8]) -> Option<String> {
    let scratch = std::env::temp_dir();
    let tag = format!(
        "coachverify-{}-{}",
        std::process::id(),
        SCRATCH.fetch_add(1, Ordering::Relaxed)
    );
    let raw = scratch.join(format!("{tag}.pdf"));
    let text = scratch.join(format!("{tag}.txt"));
    let result = (|| {
        std::fs::write(&raw, body).ok()?;
        let status = std::process::Command::new(binary)
            .arg("-q")
            .arg(&raw)
            .arg(&text)
            .status()
            .ok()?;
        if !status.success() {
            return None;
        }
        std::fs::read_to_string(&text).ok()
    })();
    std::fs::remove_file(&raw).ok();
    std::fs::remove_file(&text).ok();
    result
}

/// Fetch one row's citations across all three passes and record evidence.
async fn run_passes(
    fetcher: &Fetcher,
    row: &super::FragmentRow,
    options: &GateOptions,
) -> anyhow::Result<super::evidence::RowEvidence> {
    use census_store::clock::{Clock, SystemClock};
    let at = SystemClock.today_iso8601();
    let mut evidence = super::evidence::RowEvidence::default();
    for url in &row.source_urls {
        match fetch_text(fetcher, url, false, options).await {
            Fetched::Text(text) => evidence.absorb(&text, row, url, &at)?,
            Fetched::Robots => evidence.robots = true,
            Fetched::Failed => evidence.failed = true,
        }
    }
    if options.xhr_pass && !(evidence.found && evidence.role_near) {
        for url in &row.source_urls {
            match fetch_text(fetcher, url, true, options).await {
                Fetched::Text(text) => evidence.absorb(&text, row, url, &at)?,
                Fetched::Robots => evidence.robots = true,
                Fetched::Failed => evidence.failed = true,
            }
        }
    }
    if options.nsaa_post && !(evidence.found && evidence.role_near) {
        for url in row
            .source_urls
            .iter()
            .filter(|url| url.contains(crate::coachverify::NSAA_EXPORT_SCREEN))
        {
            match fetch_nsaa_post(fetcher, url, &row.school, options).await {
                Fetched::Text(text) => evidence.absorb(&text, row, url, &at)?,
                Fetched::Robots => evidence.robots = true,
                Fetched::Failed => evidence.failed = true,
            }
        }
    }
    Ok(evidence)
}

/// Inner loop of the gate: fetch, evaluate evidence, compute verdict per row.
pub(super) async fn verify_one_fragment(
    fetcher: &Fetcher,
    path: &std::path::Path,
    out_dir: &std::path::Path,
    options: &GateOptions,
) -> anyhow::Result<super::verdict::FragmentOutcome> {
    let rows = crate::coachverify::read_fragment(path)?;
    let mut outcomes = Vec::with_capacity(rows.len());
    let mut counts: std::collections::BTreeMap<&'static str, usize> = super::verdict::Verdict::ALL
        .iter()
        .map(|v| (v.as_str(), 0usize))
        .collect();
    for row in rows {
        let evidence = run_passes(fetcher, &row, options).await?;
        let verdict = evidence.verdict();
        if let Some(slot) = counts.get_mut(verdict.as_str()) {
            *slot = slot.saturating_add(1);
        }
        outcomes.push(super::RowOutcome {
            row,
            verdict,
            evidence: evidence.claims,
        });
    }
    let claims: Vec<&super::ClaimEvidence> =
        outcomes.iter().flat_map(|row| &row.evidence).collect();
    census_store::read::write_snapshot_rows(
        &out_dir.join(format!(
            "{}.evidence.jsonl",
            crate::coachverify::fragment_file_name(path)
        )),
        &claims,
    )?;
    crate::coachverify::write_fragment(
        &out_dir.join(crate::coachverify::fragment_file_name(path)),
        &outcomes,
    )?;
    Ok(super::verdict::FragmentOutcome {
        file: path.display().to_string(),
        rows: outcomes,
        counts,
    })
}
