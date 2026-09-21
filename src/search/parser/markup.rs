use super::{SearchCandidate, SearchIssue, SearchQuery};
use crate::domain::{
    evidence::Sport,
    identity::{EvidenceDigest, ProfileUrl},
};
use anyhow::{bail, Context, Result};
use lol_html::{html_content::Element, ElementContentHandlers, Selector, Settings};
use std::{borrow::Cow, cell::RefCell, rc::Rc};

#[path = "stream.rs"]
mod stream;

const MAX_TEXT_BYTES: usize = 8_192;

pub(super) fn rows(
    query: &SearchQuery,
    start: u32,
    digest: &EvidenceDigest,
    html: &str,
) -> Result<(Vec<SearchCandidate>, Vec<SearchIssue>)> {
    stream::rows(query, start, digest, html)
}

pub(super) fn next_offset(pager: &str, start: u32) -> Result<Option<u32>> {
    let shared = Rc::new(RefCell::new((start, None)));
    let offset_state = Rc::clone(&shared);
    let handlers =
        ElementContentHandlers::default().element(move |element: &mut Element<'_, '_>| {
            let raw = element
                .get_attribute("data-start")
                .ok_or_else(|| stream::handler_error("pager offset is absent"))?;
            let value = raw
                .parse::<u32>()
                .map_err(|error| stream::handler_error(format!("invalid pager offset: {error}")))?;
            let mut state = offset_state.borrow_mut();
            if value > state.0 {
                state.1 = Some(state.1.map_or(value, |current: u32| current.min(value)));
            }
            Ok(())
        });
    let mut settings = Settings::new();
    settings
        .element_content_handlers
        .push((selector("[data-start]")?, handlers));
    crate::html_bounds::rewrite_bounded(pager, settings)?;
    let state = Rc::try_unwrap(shared)
        .map_err(|_| anyhow::anyhow!("streaming pager parser retained callback state"))?
        .into_inner();
    Ok(state.1)
}

fn selector(value: &str) -> Result<Cow<'static, Selector>> {
    value
        .parse::<Selector>()
        .map(Cow::Owned)
        .map_err(|error| anyhow::anyhow!("invalid fixed search selector {value:?}: {error:?}"))
}

fn link_profile(href: &str) -> Result<ProfileUrl> {
    let absolute;
    let href = if href.starts_with('/') {
        absolute = format!("https://www.athletic.net{href}");
        &absolute
    } else {
        href
    };
    ProfileUrl::parse(href).context("athlete URL is not canonical")
}

fn profile_matches_sport(profile: &ProfileUrl, sport: Sport) -> bool {
    let required = match sport {
        Sport::TrackField => "/track-and-field",
        Sport::CrossCountry => "/cross-country",
    };
    profile.as_str().ends_with(required)
        || profile
            .as_str()
            .strip_suffix("/all")
            .is_some_and(|path| path.ends_with(required))
}

fn athlete_link(href: &str) -> bool {
    href.as_bytes()
        .windows(9)
        .any(|part| part.eq_ignore_ascii_case(b"/athlete/"))
}

fn normalize(raw: &str) -> Result<String> {
    let decoded = html_escape::decode_html_entities(raw).to_string();
    if decoded.len() > MAX_TEXT_BYTES {
        bail!("search row text exceeds bound");
    }
    decoded
        .split_whitespace()
        .try_fold(String::new(), |mut text, word| {
            let separator = usize::from(!text.is_empty());
            let growth = separator
                .checked_add(word.len())
                .context("search row text exceeds bound")?;
            let size = text
                .len()
                .checked_add(growth)
                .context("search row text exceeds bound")?;
            if size > MAX_TEXT_BYTES {
                bail!("search row text exceeds bound");
            }
            text.try_reserve(growth)
                .context("allocating bounded search row text")?;
            if separator != 0 {
                text.push(' ');
            }
            text.push_str(word);
            Ok(text)
        })
}
