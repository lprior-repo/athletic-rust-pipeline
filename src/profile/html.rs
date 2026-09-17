use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::domain::{
    evidence::{EvidenceIssue, EvidenceRef, Observed},
    facts::{AthleteName, GraduationYear},
    identity::{AthleteId, EvidenceDigest, ProfileUrl},
};

mod state;
mod stream;
const MAX_HTML_BYTES: usize = 32 * 1024 * 1024;

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
    let source = std::str::from_utf8(bytes).context("profile HTML is not UTF-8")?;
    let document = stream::parse(source, id)?;
    let profile_url = canonical_url(&document, id)?;
    let mut issues = Vec::new();
    let embedded_state = document.scripts;
    let (tree_hints, identity_hints) = state::parse(&embedded_state, id, &digest, &mut issues);
    let cohort_witnesses = cohort_witnesses(document.cohorts, &digest, &mut issues)?;
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

static COHORT: LazyLock<std::result::Result<Regex, String>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\bclass\s+of\s+(\d{4})\b|\bgraduat(?:ing|ion)\s+(?:class|year)\s*[:#]?\s*(\d{4})\b",
    )
    .map_err(|error| error.to_string())
});

fn canonical_url(document: &stream::Parsed, id: AthleteId) -> Result<ProfileUrl> {
    let mut urls = document
        .canonical
        .iter()
        .map(String::as_str)
        .chain(document.og_url.iter().map(String::as_str));
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

fn cohort_witnesses(
    containers: Vec<(usize, String)>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Result<Vec<Observed<GraduationYear>>> {
    let cohort = COHORT
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    containers
        .into_iter()
        .try_fold(Vec::new(), |mut years, (index, text)| {
            cohort
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
