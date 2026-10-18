use super::schema;
use crate::CrawlResult;
use html5gum::{DefaultEmitter, Token, Tokenizer};

#[derive(Default)]
struct Active {
    html: String,
    inert: Vec<&'static [u8]>,
}

pub(super) fn normalize(html: &str) -> CrawlResult<String> {
    if html.len() > crate::school_sites::MAX_PAGE_BYTES {
        return Err(schema("SIDEARM active markup exceeds 8 MiB"));
    }
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let mut tokens = Tokenizer::new_with_emitter(html.as_bytes(), emitter);
    let mut active = Active::default();
    for token in tokens.by_ref().take(65_536) {
        active.observe(token.map_err(|_| schema("SIDEARM tokenizer input failed"))?)?;
    }
    if tokens.next().is_some() || !active.inert.is_empty() {
        return Err(schema(
            "SIDEARM markup is incomplete or exceeds token budget",
        ));
    }
    Ok(active.html)
}

impl Active {
    fn observe(&mut self, token: Token) -> CrawlResult<()> {
        match token {
            Token::StartTag(tag) => self.start(tag),
            Token::EndTag(tag) => self.end(&tag.name),
            Token::String(text) if self.inert.is_empty() => self.escaped(&text, false),
            Token::Error(_) => Err(schema("SIDEARM markup is malformed")),
            _ => Ok(()),
        }
    }

    fn start(&mut self, tag: html5gum::StartTag<()>) -> CrawlResult<()> {
        if let Some(kind) = inert(&tag.name) {
            if self.inert.len() >= 128 {
                return Err(schema("SIDEARM markup exceeds inert depth budget"));
            }
            if !tag.self_closing || !matches!(kind, b"svg" | b"math") {
                self.inert.push(kind);
            }
            return Ok(());
        }
        if !self.inert.is_empty() {
            return Ok(());
        }
        self.push("<")?;
        self.escaped(&tag.name, true)?;
        for (name, value) in &tag.attributes {
            self.push(" ")?;
            self.escaped(name, true)?;
            self.push("=\"")?;
            self.escaped(value, true)?;
            self.push("\"")?;
        }
        self.push(">")
    }

    fn end(&mut self, name: &[u8]) -> CrawlResult<()> {
        if let Some(kind) = inert(name) {
            if let Some(position) = self.inert.iter().rposition(|open| *open == kind) {
                self.inert.truncate(position);
            }
        } else if self.inert.is_empty() {
            self.push("</")?;
            self.escaped(name, true)?;
            self.push(">")?;
        }
        Ok(())
    }

    fn escaped(&mut self, bytes: &[u8], attribute: bool) -> CrawlResult<()> {
        let text = std::str::from_utf8(bytes).map_err(|_| schema("SIDEARM text is not UTF-8"))?;
        for ch in text.chars() {
            match ch {
                '<' => self.push("&lt;")?,
                '>' => self.push("&gt;")?,
                '&' => self.push("&amp;")?,
                '"' if attribute => self.push("&quot;")?,
                _ => self.push(ch.encode_utf8(&mut [0; 4]))?,
            }
        }
        Ok(())
    }

    fn push(&mut self, text: &str) -> CrawlResult<()> {
        let length = self
            .html
            .len()
            .checked_add(text.len())
            .ok_or_else(|| schema("SIDEARM normalized size overflow"))?;
        if length > 8 * 1024 * 1024 {
            return Err(schema("SIDEARM normalized markup exceeds 8 MiB"));
        }
        self.html
            .try_reserve(text.len())
            .map_err(|_| schema("SIDEARM normalized markup allocation failed"))?;
        self.html.push_str(text);
        Ok(())
    }
}

fn inert(name: &[u8]) -> Option<&'static [u8]> {
    crate::coach_directories::inert_markup_kind(name).filter(|kind| *kind != b"script")
}
