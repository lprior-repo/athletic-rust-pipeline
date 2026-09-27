use census_crawl::net::{FetchError, FetchOptions, Fetcher};
use census_domain::model::RawContactRow;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct GateOptions {
    pub xhr_pass: bool,
    pub nsaa_post: bool,
    pub pdftotext: Option<String>,
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

enum Fetched {
    Text {
        text: String,
        sha256: String,
        fetched_at: String,
    },
    Robots,
    Failed,
}

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
        Ok(outcome) => Fetched::Text {
            text: body_text(&outcome.body, options.pdftotext.as_deref()),
            sha256: outcome.content_digest,
            fetched_at: outcome.fetched_at,
        },
        Err(FetchError::Robots(_)) => Fetched::Robots,
        Err(_) => Fetched::Failed,
    }
}

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
        Ok(outcome) => Fetched::Text {
            text: body_text(&outcome.body, options.pdftotext.as_deref()),
            sha256: outcome.content_digest,
            fetched_at: outcome.fetched_at,
        },
        Err(FetchError::Robots(_)) => Fetched::Robots,
        Err(_) => Fetched::Failed,
    }
}

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

static SCRATCH: AtomicU64 = AtomicU64::new(0);

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

async fn run_passes(
    fetcher: &Fetcher,
    row: &RawContactRow,
    options: &GateOptions,
) -> anyhow::Result<super::evidence::RowEvidence> {
    let mut evidence = super::evidence::RowEvidence::default();
    for url in &row.source_urls {
        match fetch_text(fetcher, url, false, options).await {
            Fetched::Text {
                text,
                sha256,
                fetched_at,
            } => evidence.absorb(&text, row, url, &fetched_at, &sha256)?,
            Fetched::Robots => evidence.robots = true,
            Fetched::Failed => evidence.failed = true,
        }
    }
    if options.xhr_pass && !(evidence.found && evidence.role_near) {
        for url in &row.source_urls {
            match fetch_text(fetcher, url, true, options).await {
                Fetched::Text {
                    text,
                    sha256,
                    fetched_at,
                } => evidence.absorb(&text, row, url, &fetched_at, &sha256)?,
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
                Fetched::Text {
                    text,
                    sha256,
                    fetched_at,
                } => evidence.absorb(&text, row, url, &fetched_at, &sha256)?,
                Fetched::Robots => evidence.robots = true,
                Fetched::Failed => evidence.failed = true,
            }
        }
    }
    Ok(evidence)
}

pub(super) async fn verify_one_fragment(
    fetcher: &Fetcher,
    path: &std::path::Path,
    out_dir: &std::path::Path,
    options: &GateOptions,
) -> anyhow::Result<super::verdict::FragmentOutcome> {
    let rows = crate::coachverify::read_fragment(path)?;
    let mut outcomes = Vec::with_capacity(rows.len());
    let mut counts: std::collections::BTreeMap<super::Verdict, usize> =
        super::verdict::Verdict::ALL
            .iter()
            .map(|v| (*v, 0usize))
            .collect();
    for row in rows {
        let evidence = run_passes(fetcher, &row, options).await?;
        let verdict = evidence.verdict();
        let count = counts.entry(verdict).or_default();
        *count = count.saturating_add(1);
        outcomes.push(super::RowOutcome {
            row,
            verdict,
            evidence: evidence.claims,
        });
    }
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
