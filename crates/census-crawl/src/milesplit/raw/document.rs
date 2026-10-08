use super::{schema, SportsEvent};
use crate::milesplit::parse::markup::{Declaration, Markup};
use crate::CrawlResult;
use html5gum::Tokenizer;
use serde::Deserialize;
use serde_json::Value;

mod nodes;

pub(super) enum Ownership<'a> {
    Unbound,
    Bound(&'a dyn Fn(&str, &str) -> CrawlResult<()>),
}

pub(super) fn read(html: &str, url: &str, ownership: Ownership<'_>) -> CrawlResult<SportsEvent> {
    if html.len() > crate::net::MAX_BODY_BYTES {
        return Err(crate::net::FetchError::TooLarge { url: url.into() }.into());
    }
    let event = Tokenizer::new_with_emitter(html, Markup::default()).try_fold(
        None,
        |selected, token| {
            let token =
                token.map_err(|error| schema(url, &format!("raw HTML input failed: {error}")))?;
            match token {
                Declaration::Script(bytes) => script(&bytes, selected, url, &ownership),
                Declaration::Url(bytes) => {
                    if let Ownership::Bound(check) = &ownership {
                        let published = std::str::from_utf8(&bytes)
                            .map_err(|_| schema(url, "published raw document URL is not UTF-8"))?;
                        check(published, "canonical raw document URL")?;
                    }
                    Ok(selected)
                }
                Declaration::Invalid => Err(schema(url, "invalid active raw HTML declaration")),
            }
        },
    )?;
    event.ok_or_else(|| schema(url, "raw metadata has no published SportsEvent document"))
}

fn script(
    bytes: &[u8],
    selected: Option<SportsEvent>,
    url: &str,
    ownership: &Ownership<'_>,
) -> CrawlResult<Option<SportsEvent>> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| schema(url, &format!("raw JSON-LD did not decode: {error}")))?;
    let answer = nodes::walk(&value).try_fold(selected, |selected, node| {
        let node = node?;
        let sports_event = has_kind(node, "SportsEvent");
        let legacy = matches!(ownership, Ownership::Unbound)
            && node.get("name").is_some()
            && node.get("startDate").is_some();
        if !(sports_event || legacy) {
            if has_kind(node, "WebPage") {
                check_owner(node, url, ownership)?;
            }
            return Ok(selected);
        }
        check_owner(node, url, ownership)?;
        let event = SportsEvent::deserialize(node).map_err(|error| {
            schema(
                url,
                &format!("published SportsEvent did not decode: {error}"),
            )
        })?;
        match selected {
            Some(prior) if prior != event => {
                Err(schema(url, "conflicting active SportsEvent metadata"))
            }
            Some(prior) => Ok(Some(prior)),
            None => Ok(Some(event)),
        }
    });
    answer
}

fn check_owner(node: &Value, url: &str, ownership: &Ownership<'_>) -> CrawlResult<()> {
    let Ownership::Bound(check) = ownership else {
        return Ok(());
    };
    let found =
        ["url", "@id"]
            .into_iter()
            .try_fold(false, |found, field| -> CrawlResult<bool> {
                let Some(value) = node.get(field) else {
                    return Ok(found);
                };
                let published = value
                    .as_str()
                    .ok_or_else(|| schema(url, "published raw owner URL is not text"))?;
                check(published, "published raw document URL")?;
                Ok(true)
            })?;
    if !found {
        return Err(schema(url, "published raw document has no owner URL"));
    }
    Ok(())
}

fn has_kind(node: &Value, kind: &str) -> bool {
    let matches = |value: &Value| {
        value.as_str().is_some_and(|value| {
            value == kind
                || value.strip_prefix("https://schema.org/") == Some(kind)
                || value.strip_prefix("http://schema.org/") == Some(kind)
        })
    };
    match node.get("@type") {
        Some(Value::Array(kinds)) => kinds.iter().any(matches),
        Some(value) => matches(value),
        None => false,
    }
}
