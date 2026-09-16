use anyhow::{bail, Context, Result};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::domain::{
    evidence::{EvidenceIssue, EvidenceRef, Observed},
    facts::{AthleteName, GraduationYear},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};

mod state;
const MAX_HTML_BYTES: usize = 32 * 1024 * 1024;
const MAX_SCRIPT_BYTES: usize = 2 * 1024 * 1024;
const MAX_SCRIPTS: usize = 1024;
const MAX_DOM_DEPTH: usize = 128;
const MAX_DOM_NODES: usize = 500_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeHint {
    pub kind: String,
    pub id: Option<u64>,
    pub label: Option<String>,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HtmlProfileEvidence {
    pub athlete_id: AthleteId,
    pub profile_url: ProfileUrl,
    pub cohort_witnesses: Vec<Observed<GraduationYear>>,
    pub tree_hints: Vec<TreeHint>,
    pub identity_hints: Vec<Observed<AthleteName>>,
    pub issues: Vec<EvidenceIssue>,
    pub document: EvidenceDigest,
    pub embedded_state: Vec<String>,
}

pub fn parse_profile_html(
    id: AthleteId,
    digest: EvidenceDigest,
    bytes: &[u8],
) -> Result<HtmlProfileEvidence> {
    if bytes.len() > MAX_HTML_BYTES {
        bail!("profile HTML exceeds parser byte bound");
    }
    let document =
        Html::parse_document(std::str::from_utf8(bytes).context("profile HTML is not UTF-8")?);
    if document.tree.nodes().count() > MAX_DOM_NODES {
        bail!("profile DOM exceeds node bound");
    }
    let selectors = selectors()?;
    let profile_url = canonical_url(&document, selectors, id)?;
    let mut issues = Vec::new();
    let embedded_state = scripts(&document, selectors)?;
    let (tree_hints, identity_hints) = state::parse(&embedded_state, id, &digest, &mut issues);
    let cohort_witnesses = cohort_witnesses(&document, selectors, id, &digest, &mut issues)?;
    Ok(HtmlProfileEvidence {
        athlete_id: id,
        profile_url,
        cohort_witnesses,
        tree_hints,
        identity_hints,
        issues,
        document: digest,
        embedded_state,
    })
}

struct Selectors {
    canonical: Selector,
    og_url: Selector,
    scripts: Selector,
    athlete: Selector,
    cohort: Regex,
}

static SELECTORS: LazyLock<std::result::Result<Selectors, String>> = LazyLock::new(|| {
    Ok(Selectors {
            canonical: selector("link[rel~='canonical'][href]")?,
            og_url: selector("meta[property='og:url'][content], meta[name='og:url'][content]")?,
            scripts: selector("script")?,
            athlete: selector("[data-athlete-id]")?,
            cohort: Regex::new(r"(?i)\bclass\s+of\s+(\d{4})\b|\bgraduat(?:ing|ion)\s+(?:class|year)\s*[:#]?\s*(\d{4})\b")
                .map_err(|error| error.to_string())?,
        })
});

fn selectors() -> Result<&'static Selectors> {
    match &*SELECTORS {
        Ok(value) => Ok(value),
        Err(message) => bail!("invalid fixed profile selectors: {message}"),
    }
}

fn selector(value: &str) -> std::result::Result<Selector, String> {
    Selector::parse(value).map_err(|error| format!("{error:?}"))
}

fn canonical_url(document: &Html, selectors: &Selectors, id: AthleteId) -> Result<ProfileUrl> {
    let mut urls = document
        .select(&selectors.canonical)
        .filter_map(|node| node.attr("href"))
        .chain(
            document
                .select(&selectors.og_url)
                .filter_map(|node| node.attr("content")),
        );
    let first = urls
        .next()
        .context("profile HTML has no canonical identity URL")?;
    let first = ProfileUrl::parse(first)?;
    if first.athlete_id() != id {
        bail!("canonical HTML athlete ID differs from request");
    }
    urls.try_for_each(|raw| -> Result<()> {
        if ProfileUrl::parse(raw)?.athlete_id() != id {
            bail!("conflicting HTML identity URLs");
        }
        Ok(())
    })?;
    Ok(first)
}

fn scripts(document: &Html, selectors: &Selectors) -> Result<Vec<String>> {
    document.select(&selectors.scripts).enumerate().try_fold(
        Vec::new(),
        |mut scripts, (index, node)| {
            if index >= MAX_SCRIPTS {
                bail!("profile script count exceeds bound");
            }
            let size = node
                .text()
                .try_fold(0_usize, |size, text| size.checked_add(text.len()))
                .context("profile script byte count overflow")?;
            if size > MAX_SCRIPT_BYTES {
                bail!("profile script exceeds parsing bound; full body retained as raw artifact");
            }
            scripts.push(node.text().collect());
            Ok(scripts)
        },
    )
}

fn cohort_witnesses(
    document: &Html,
    selectors: &Selectors,
    id: AthleteId,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Result<Vec<Observed<GraduationYear>>> {
    document.select(&selectors.athlete).enumerate().try_fold(
        Vec::new(),
        |mut years, (index, container)| {
            if container
                .attr("data-athlete-id")
                .and_then(|raw| raw.parse::<u64>().ok())
                != Some(id.get())
            {
                return Ok(years);
            }
            let text = scoped_text(container)?;
            selectors
                .cohort
                .captures_iter(&text)
                .try_for_each(|capture| -> Result<()> {
                    let found = capture
                        .get(1)
                        .or_else(|| capture.get(2))
                        .context("cohort pattern lacks year capture")?;
                    let reference = evidence(
                        digest,
                        format!(
                            "html/athlete-container/{index}/text/{}..{}",
                            found.start(),
                            found.end()
                        ),
                    );
                    match found
                        .as_str()
                        .parse::<u16>()
                        .ok()
                        .and_then(|year| GraduationYear::new(year).ok())
                    {
                        Some(year) => years.push(Observed {
                            value: year,
                            evidence: reference,
                        }),
                        None => issues.push(EvidenceIssue {
                            code: "invalid_graduation_year".to_owned(),
                            message:
                                "optional graduation observation is outside the supported range"
                                    .to_owned(),
                            evidence: Some(reference),
                        }),
                    }
                    Ok(())
                })?;
            Ok(years)
        },
    )
}

fn scoped_text(container: ElementRef<'_>) -> Result<String> {
    container
        .descendants()
        .try_fold(String::new(), |mut text, node| {
            let Some(value) = node.value().as_text() else {
                return Ok(text);
            };
            let (owner, excluded) = node
                .ancestors()
                .filter_map(ElementRef::wrap)
                .enumerate()
                .try_fold(
                    (None, false),
                    |(owner, excluded), (depth, element)| -> Result<_> {
                        if depth >= MAX_DOM_DEPTH {
                            bail!("profile text ancestry exceeds bound");
                        }
                        let owner = owner.or_else(|| {
                            element
                                .attr("data-athlete-id")
                                .map(|_| element.id() == container.id())
                        });
                        let excluded = excluded
                            || matches!(
                                element.value().name(),
                                "script" | "style" | "template" | "noscript"
                            )
                            || element.attr("hidden").is_some();
                        Ok((owner, excluded))
                    },
                )?;
            if !excluded && owner == Some(true) {
                text.push_str(value);
                text.push(' ');
            }
            Ok(text)
        })
}

fn evidence(digest: &EvidenceDigest, locator: impl Into<String>) -> EvidenceRef {
    EvidenceRef {
        document: digest.clone(),
        locator: locator.into(),
    }
}

fn issue(
    code: &str,
    message: &str,
    digest: &EvidenceDigest,
    locator: impl Into<String>,
) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence: Some(evidence(digest, locator)),
    }
}
