use census_domain::model::ContactResearchOutcome as Outcome;
use html5gum::{DefaultEmitter, Token, Tokenizer};

type ParseResult<T> = Result<T, Failure>;

#[derive(Debug, thiserror::Error)]
pub(super) enum Failure {
    #[error("{0}")]
    Budget(&'static str),
    #[error("{0}")]
    Decode(&'static str),
}

impl Failure {
    pub(super) fn outcome(&self) -> Outcome {
        match self {
            Self::Budget(_) => Outcome::Partial,
            Self::Decode(_) => Outcome::Failed,
        }
    }
}

#[derive(Default)]
pub(super) struct Page {
    pub owner: Option<String>,
    pub ambiguous: bool,
    pub malformed: bool,
    pub limited: bool,
    pub sidearm: bool,
    pub blocks: Vec<Block>,
    pub links: Vec<String>,
    inert: Vec<&'static [u8]>,
    current: Option<Block>,
}

#[derive(Default)]
pub(super) struct Block {
    pub text: String,
    pub mailboxes: Vec<String>,
}

pub(super) fn parse(body: &[u8]) -> ParseResult<Page> {
    if body.len() > 1024 * 1024 {
        return Err(error("official school contact page exceeds 1 MiB"));
    }
    std::str::from_utf8(body)
        .map_err(|_| Failure::Decode("school contact page bytes are not UTF-8"))?;
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let mut page = Page::default();
    let mut tokens = Tokenizer::new_with_emitter(body, emitter);
    for token in tokens.by_ref().take(65_536) {
        match page
            .observe(token.map_err(|_| Failure::Decode("school contact tokenizer input failed"))?)
        {
            Ok(()) => {}
            Err(Failure::Budget(_)) => {
                page.limited = true;
                page.current = None;
                break;
            }
            Err(error) => return Err(error),
        }
    }
    if tokens.next().is_some() {
        page.limited = true;
        page.current = None;
    }
    page.malformed |= !page.inert.is_empty();
    page.finish()?;
    Ok(page)
}

impl Page {
    fn observe(&mut self, token: Token) -> ParseResult<()> {
        match token {
            Token::StartTag(tag) => self.start(tag),
            Token::EndTag(tag) => self.end(&tag.name),
            Token::String(text) if self.inert.is_empty() => self.text(&text),
            Token::Error(_) => {
                self.malformed = true;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn start(&mut self, tag: html5gum::StartTag<()>) -> ParseResult<()> {
        if let Some(kind) = inert_kind(&tag.name) {
            if self.inert.len() >= 128 {
                return Err(error("school contact page exceeds 128 inert scopes"));
            }
            if !tag.self_closing || !matches!(kind, b"svg" | b"math") {
                self.inert.push(kind);
            }
            return Ok(());
        }
        if !self.inert.is_empty() {
            return Ok(());
        }
        match tag.name.as_ref() {
            b"meta" if attribute(&tag, b"property") == Some("og:site_name") => {
                self.owner(attribute(&tag, b"content"))
            }
            b"p" | b"address" | b"li" | b"td" => {
                self.finish()?;
                self.current = Some(Block::default());
            }
            b"a" => self.anchor(attribute(&tag, b"href"))?,
            b"article" => {
                self.sidearm |= attribute(&tag, b"class").is_some_and(|classes| {
                    classes
                        .split_ascii_whitespace()
                        .any(|class| class == "sidearm-staff")
                })
            }
            b"br" => self.text(b" ")?,
            _ => {}
        }
        Ok(())
    }

    fn end(&mut self, name: &[u8]) -> ParseResult<()> {
        if let Some(kind) = inert_kind(name) {
            if let Some(position) = self.inert.iter().rposition(|open| *open == kind) {
                self.inert.truncate(position);
            }
        } else if self.inert.is_empty() && matches!(name, b"p" | b"address" | b"li" | b"td") {
            self.finish()?;
        }
        Ok(())
    }

    fn owner(&mut self, owner: Option<&str>) {
        if let Some(owner) = owner {
            if owner.len() > 512 {
                self.ambiguous = true;
                return;
            }
            if self
                .owner
                .as_deref()
                .is_some_and(|previous| previous != owner)
            {
                self.ambiguous = true;
            } else if self.owner.is_none() {
                self.owner = Some(owner.to_owned());
            }
        }
    }

    fn anchor(&mut self, href: Option<&str>) -> ParseResult<()> {
        let Some(href) = href else {
            return Ok(());
        };
        if href.len() > 4096 {
            return Err(error("school contact link exceeds 4096 bytes"));
        }
        if let Some(address) = super::after(href, "mailto:") {
            if let Some(block) = &mut self.current {
                if block.mailboxes.len() >= 32 {
                    return Err(error("school office block exceeds 32 mailboxes"));
                }
                block.mailboxes.push(
                    address
                        .split('?')
                        .next()
                        .map_or(address, |address| address)
                        .to_owned(),
                );
            }
        } else if ["contact", "staff", "directory"].iter().any(|needle| {
            href.as_bytes()
                .windows(needle.len())
                .any(|part| part.eq_ignore_ascii_case(needle.as_bytes()))
        }) {
            if self.links.len() >= 128 {
                return Err(error(
                    "school contact page exceeds 128 published contact links",
                ));
            }
            self.links.push(href.to_owned());
        }
        Ok(())
    }

    fn text(&mut self, bytes: &[u8]) -> ParseResult<()> {
        let Some(block) = &mut self.current else {
            return Ok(());
        };
        let text = std::str::from_utf8(bytes)
            .map_err(|_| Failure::Decode("school contact page text is not UTF-8"))?;
        if block
            .text
            .len()
            .checked_add(text.len())
            .is_none_or(|len| len > 4096)
        {
            return Err(error("school contact block exceeds 4096 bytes"));
        }
        block.text.push_str(text);
        Ok(())
    }

    fn finish(&mut self) -> ParseResult<()> {
        if let Some(block) = self.current.take() {
            if !block.mailboxes.is_empty() {
                if self.blocks.len() >= 4096 {
                    return Err(error("school contact page exceeds 4096 mailbox blocks"));
                }
                self.blocks.push(block);
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

fn error(detail: &'static str) -> Failure {
    Failure::Budget(detail)
}
