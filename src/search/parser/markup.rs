use super::{SearchCandidate, SearchIssue, SearchQuery};
use crate::domain::{
    evidence::{EvidenceRef, Sport},
    identity::{EvidenceDigest, ProfileUrl},
};
use anyhow::{bail, Context, Result};
use html5ever::{local_name, ns, tendril::TendrilSink, QualName};
use scraper::{ElementRef, Html, HtmlTreeSink, Selector};
use std::sync::LazyLock;

const MAX_ROWS: usize = 1000;
const MAX_DOM_NODES: usize = 500_000;

struct Selectors {
    rows: Selector,
    links: Selector,
    offsets: Selector,
}
static SELECTORS: LazyLock<std::result::Result<Selectors, String>> = LazyLock::new(|| {
    let parse = |value| Selector::parse(value).map_err(|error| format!("{error:?}"));
    Ok(Selectors {
        rows: parse("tr")?,
        links: parse("a[href]")?,
        offsets: parse("[data-start]")?,
    })
});

fn selectors() -> Result<&'static Selectors> {
    SELECTORS
        .as_ref()
        .map_err(|message| anyhow::anyhow!("invalid fixed search selectors: {message}"))
}

pub(super) fn rows(
    query: &SearchQuery,
    start: u32,
    digest: &EvidenceDigest,
    html: &str,
) -> Result<(Vec<SearchCandidate>, Vec<SearchIssue>)> {
    let document = html5ever::parse_fragment(
        HtmlTreeSink::new(Html::new_fragment()),
        Default::default(),
        QualName::new(None, ns!(html), local_name!("tbody")),
        Vec::new(),
        false,
    )
    .one(html);
    if document.tree.nodes().count() > MAX_DOM_NODES {
        bail!("search DOM exceeds node bound");
    }
    let selectors = selectors()?;
    let mut candidates = Vec::new();
    let mut issues = Vec::new();
    document
        .select(&selectors.rows)
        .enumerate()
        .try_for_each(|(index, row)| -> Result<()> {
            if index >= MAX_ROWS {
                bail!("search page exceeds row bound");
            }
            let evidence = EvidenceRef {
                document: digest.clone(),
                locator: format!("/d/results/page/{start}/row/{index}"),
            };
            match row_candidate(query.sport, row, selectors, evidence.clone()) {
                Ok(Some(candidate)) => candidates.push(candidate),
                Ok(None) => {}
                Err(error) => issues.push(SearchIssue {
                    code: "invalid_athlete_row".into(),
                    message: error.to_string(),
                    evidence,
                }),
            }
            Ok(())
        })?;
    let outside_rows = document.select(&selectors.links).any(|link| {
        athlete_link(link)
            && !link
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|element| element.value().name() == "tr")
    });
    if outside_rows {
        issues.push(SearchIssue {
            code: "unrecognized_result_layout".into(),
            message: "athlete link outside a result row".into(),
            evidence: EvidenceRef {
                document: digest.clone(),
                locator: "/d/results".into(),
            },
        });
    }
    Ok((candidates, issues))
}

fn row_candidate(
    sport: Sport,
    row: ElementRef<'_>,
    selectors: &Selectors,
    evidence: EvidenceRef,
) -> Result<Option<SearchCandidate>> {
    let required = match sport {
        Sport::TrackField => "/track-and-field",
        Sport::CrossCountry => "/cross-country",
    };
    let mut identity = None;
    let mut selected = None;
    let mut display_name = String::new();
    row.select(&selectors.links)
        .filter(|link| athlete_link(*link))
        .enumerate()
        .try_for_each(|(index, link)| -> Result<()> {
            if index >= 16 {
                bail!("result row exceeds athlete link bound");
            }
            let profile = link_profile(link)?;
            if identity.is_some_and(|id| id != profile.athlete_id()) {
                bail!("result row has conflicting athlete identity links");
            }
            identity = Some(profile.athlete_id());
            let name = normalize(link.text().flat_map(str::split_whitespace))?;
            if !name.is_empty() {
                if !display_name.is_empty() && display_name != name {
                    bail!("result row has conflicting display names");
                }
                display_name = name;
            }
            if profile.as_str().ends_with(required)
                || profile
                    .as_str()
                    .strip_suffix("/all")
                    .is_some_and(|path| path.ends_with(required))
            {
                selected = Some(profile);
            }
            Ok(())
        })?;
    let Some(athlete_id) = identity else {
        return Ok(None);
    };
    let profile_url = selected.context("result row belongs to another or unspecified sport")?;
    if display_name.is_empty() || display_name.len() > 512 {
        bail!("athlete display name is absent or exceeds bound");
    }
    let snippet = normalize(row.text().flat_map(str::split_whitespace))?;
    Ok(Some(SearchCandidate {
        athlete_id,
        profile_url,
        display_name,
        snippet,
        sport,
        evidence,
    }))
}

fn link_profile(link: ElementRef<'_>) -> Result<ProfileUrl> {
    let href = link.attr("href").context("athlete link lacks href")?;
    let absolute;
    let href = if href.starts_with('/') {
        absolute = format!("https://www.athletic.net{href}");
        &absolute
    } else {
        href
    };
    ProfileUrl::parse(href).context("athlete URL is not canonical")
}

fn athlete_link(link: ElementRef<'_>) -> bool {
    link.attr("href").is_some_and(|href| {
        href.as_bytes()
            .windows(9)
            .any(|part| part.eq_ignore_ascii_case(b"/athlete/"))
    })
}

fn normalize<'a>(mut words: impl Iterator<Item = &'a str>) -> Result<String> {
    words.try_fold(String::new(), |mut text, word| {
        let separator = usize::from(!text.is_empty());
        let size = text
            .len()
            .checked_add(separator)
            .and_then(|size| size.checked_add(word.len()))
            .filter(|size| *size <= 8_192)
            .context("search row text exceeds bound")?;
        text.try_reserve(size - text.len())
            .context("allocating bounded search row text")?;
        if separator != 0 {
            text.push(' ');
        }
        text.push_str(word);
        Ok(text)
    })
}

pub(super) fn next_offset(pager: &str, start: u32) -> Result<Option<u32>> {
    let document = Html::parse_fragment(pager);
    if document.tree.nodes().count() > MAX_DOM_NODES {
        bail!("search pager exceeds node bound");
    }
    document
        .select(&selectors()?.offsets)
        .try_fold(None, |minimum: Option<u32>, node| {
            let value = node
                .attr("data-start")
                .context("pager offset is absent")?
                .parse::<u32>()
                .context("invalid pager offset")?;
            Ok(if value > start {
                Some(minimum.map_or(value, |current| current.min(value)))
            } else {
                minimum
            })
        })
}
