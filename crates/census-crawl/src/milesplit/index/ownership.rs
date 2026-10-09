use super::{schema, CrawlError, CrawlResult};
use html5gum::{DefaultEmitter, Token, Tokenizer};

const MAX_TOKENS: usize = 65_536;
const MAX_ASSET_URL: usize = 2_048;

#[derive(Default)]
struct Surface {
    inert: Vec<&'static [u8]>,
    declaration: Option<Declaration>,
    network_asset: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Declaration {
    MileSplit,
    Other,
}

pub(super) fn validate(body: &str, url: &str) -> CrawlResult<()> {
    let parsed = url::Url::parse(url).map_err(|_| schema(url, "index URL is malformed"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || !parsed.host_str().is_some_and(provider_host)
    {
        return Err(schema(
            url,
            "index URL does not identify a MileSplit source",
        ));
    }
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let mut tokens = Tokenizer::new_with_emitter(body, emitter);
    let mut surface = Surface::default();
    tokens.by_ref().take(MAX_TOKENS).try_for_each(|token| {
        surface.observe(
            token.map_err(|_| schema(url, "index HTML input failed"))?,
            url,
        )
    })?;
    if tokens.next().is_some() {
        return Err(CrawlError::Resource {
            resource: "index HTML tokens",
            requested: MAX_TOKENS.saturating_add(1),
            limit: MAX_TOKENS,
        });
    }
    if surface.declaration == Some(Declaration::MileSplit) || surface.network_asset {
        return Ok(());
    }
    if surface.declaration == Some(Declaration::Other) {
        return Err(schema(
            url,
            "index publishes conflicting application ownership",
        ));
    }
    Err(schema(
        url,
        "index lacks its MileSplit application ownership",
    ))
}

fn provider_host(host: &str) -> bool {
    matches!(host, "milesplit.com" | "www.milesplit.com")
        || host
            .strip_suffix(".milesplit.com")
            .is_some_and(|code| census_domain::UsJurisdiction::from_code(code).is_some())
}

fn network_asset(target: &str) -> bool {
    target.len() <= MAX_ASSET_URL
        && url::Url::parse(target).is_ok_and(|parsed| {
            matches!(parsed.scheme(), "http" | "https")
                && parsed.username().is_empty()
                && parsed.password().is_none()
                && parsed.port().is_none()
                && parsed.host_str().is_some_and(network_host)
        })
}

fn network_host(host: &str) -> bool {
    host == "milesplit.com" || host.ends_with(".milesplit.com")
}

impl Surface {
    fn observe(&mut self, token: Token, url: &str) -> CrawlResult<()> {
        match token {
            Token::StartTag(tag) => self.start(tag, url),
            Token::EndTag(tag) => {
                if let Some(kind) = inert_kind(&tag.name) {
                    if let Some(position) = self.inert.iter().rposition(|open| *open == kind) {
                        self.inert.truncate(position);
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn start(&mut self, tag: html5gum::StartTag<()>, url: &str) -> CrawlResult<()> {
        if let Some(kind) = inert_kind(&tag.name) {
            if self.inert.len() >= 128 {
                return Err(schema(url, "index inert nesting exceeds 128 scopes"));
            }
            if !tag.self_closing || !matches!(kind, b"svg" | b"math") {
                self.inert
                    .try_reserve(1)
                    .map_err(|_| schema(url, "index scope allocation failed"))?;
                self.inert.push(kind);
            }
        } else if self.inert.is_empty() {
            if tag.name.as_ref() == b"meta"
                && attribute(&tag, b"name")
                    .is_some_and(|name| name.eq_ignore_ascii_case("application-name"))
            {
                let declaration = if attribute(&tag, b"content").is_some_and(|content| {
                    content
                        .split_whitespace()
                        .any(|word| word.eq_ignore_ascii_case("MileSplit"))
                }) {
                    Declaration::MileSplit
                } else {
                    Declaration::Other
                };
                if self.declaration.is_some_and(|seen| seen != declaration) {
                    return Err(schema(
                        url,
                        "index publishes conflicting application ownership",
                    ));
                }
                self.declaration = Some(declaration);
            } else if matches!(tag.name.as_ref(), b"link" | b"script") {
                let target = if tag.name.as_ref() == b"link" {
                    attribute(&tag, b"href")
                } else {
                    attribute(&tag, b"src")
                };
                if target.is_some_and(network_asset) {
                    self.network_asset = true;
                }
            }
        }
        Ok(())
    }
}

fn attribute<'a>(tag: &'a html5gum::StartTag<()>, name: &[u8]) -> Option<&'a str> {
    tag.attributes
        .get(name)
        .and_then(|value| std::str::from_utf8(value).ok())
}

fn inert_kind(name: &[u8]) -> Option<&'static [u8]> {
    match name {
        b"template" => Some(b"template"),
        b"svg" => Some(b"svg"),
        b"math" => Some(b"math"),
        b"script" => Some(b"script"),
        b"style" => Some(b"style"),
        b"noscript" => Some(b"noscript"),
        b"textarea" => Some(b"textarea"),
        b"title" => Some(b"title"),
        _ => None,
    }
}
