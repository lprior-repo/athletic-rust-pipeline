use super::{Scope, Sink};
use crate::report::ReportResult;

mod element;
mod entities;

pub(super) use element::{Element, Handler};

use element::trim;
use entities::decode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Text,
    Markup,
    Comment,
    Instruction,
    Declaration,
}

pub(super) struct Xml<'a, H> {
    handler: H,
    scope: Scope<'a>,
    budget: usize,
    token: Vec<u8>,
    text: Vec<u8>,
    name: String,
    attributes: Vec<(String, String)>,
    mode: Mode,
    quote: Option<u8>,
    closing: bool,
    empty: bool,
}

impl<'a, H: Handler> Xml<'a, H> {
    pub(super) fn new(handler: H, scope: Scope<'a>, budget: usize) -> Self {
        Self {
            handler,
            scope,
            budget,
            token: Vec::new(),
            text: Vec::new(),
            name: String::new(),
            attributes: Vec::new(),
            mode: Mode::Text,
            quote: None,
            closing: false,
            empty: false,
        }
    }

    pub(super) fn finish(mut self) -> ReportResult<H> {
        if self.mode != Mode::Text {
            return Err(self.scope.corrupt("the XML part ends inside markup"));
        }
        self.flush_text()?;
        Ok(self.handler)
    }

    fn feed(&mut self, data: &[u8]) -> ReportResult<()> {
        for &byte in data {
            match self.mode {
                Mode::Text => {
                    if byte == b'<' {
                        self.flush_text()?;
                        self.mode = Mode::Markup;
                        self.token.clear();
                        self.quote = None;
                    } else {
                        self.push_text(byte)?;
                    }
                }
                Mode::Markup => self.feed_markup(byte)?,
                Mode::Comment => {
                    self.push_token(byte)?;
                    if self.token.ends_with(b"-->") {
                        self.token.clear();
                        self.mode = Mode::Text;
                    }
                }
                Mode::Instruction => {
                    self.push_token(byte)?;
                    if self.token.ends_with(b"?>") {
                        self.token.clear();
                        self.mode = Mode::Text;
                    }
                }
                Mode::Declaration => {
                    self.push_token(byte)?;
                    if byte == b'>' {
                        self.token.clear();
                        self.mode = Mode::Text;
                    }
                }
            }
        }
        Ok(())
    }

    fn feed_markup(&mut self, byte: u8) -> ReportResult<()> {
        if let Some(quote) = self.quote {
            self.push_token(byte)?;
            if byte == quote {
                self.quote = None;
            }
            return Ok(());
        }
        match byte {
            b'"' | b'\'' => {
                self.quote = Some(byte);
                self.push_token(byte)?;
            }
            b'>' => {
                self.mode = Mode::Text;
                self.finish_tag()?;
            }
            _ => {
                self.push_token(byte)?;
                if self.token.starts_with(b"!--") {
                    self.token.clear();
                    self.mode = Mode::Comment;
                } else if self.token.as_slice() == b"![" {
                    return Err(self
                        .scope
                        .outside("CDATA sections are not part of the inline-string profile"));
                } else if self.token.starts_with(b"?") {
                    self.token.clear();
                    self.mode = Mode::Instruction;
                } else if self.token.starts_with(b"!")
                    && !self.token.starts_with(b"!-")
                    && self.token.len() > 1
                {
                    self.token.clear();
                    self.mode = Mode::Declaration;
                }
            }
        }
        Ok(())
    }

    fn push_token(&mut self, byte: u8) -> ReportResult<()> {
        if self.token.len() >= self.budget {
            return Err(self.scope.corrupt(format!(
                "an XML token exceeds the {} byte budget",
                self.budget
            )));
        }
        self.token.push(byte);
        Ok(())
    }

    fn push_text(&mut self, byte: u8) -> ReportResult<()> {
        if self.text.len() >= self.budget {
            return Err(self.scope.corrupt(format!(
                "an XML text run exceeds the {} byte budget",
                self.budget
            )));
        }
        self.text.push(byte);
        Ok(())
    }

    fn flush_text(&mut self) -> ReportResult<()> {
        if self.text.is_empty() {
            return Ok(());
        }
        let scope = self.scope;
        let text = decode(&self.text, &scope)?;
        self.text.clear();
        if text.is_empty() {
            return Ok(());
        }
        self.handler.text(&text)
    }

    fn finish_tag(&mut self) -> ReportResult<()> {
        let scope = self.scope;
        let token = std::mem::take(&mut self.token);
        let parsed = self.parse_tag(&token, &scope);
        self.token = token;
        self.token.clear();
        parsed
    }

    fn parse_tag(&mut self, token: &[u8], scope: &Scope<'_>) -> ReportResult<()> {
        self.attributes.clear();
        let mut body = trim(token);
        self.closing = body.starts_with(b"/");
        if self.closing {
            body = trim(body.get(1..).map_or(&[], |tail| tail));
        }
        self.empty = body.ends_with(b"/");
        if self.empty {
            body = trim(
                body.get(..body.len().saturating_sub(1))
                    .map_or(&[], |head| head),
            );
        }
        if body.is_empty() {
            return Err(scope.corrupt("an XML tag has no name"));
        }
        let split = body
            .iter()
            .position(|byte| byte.is_ascii_whitespace())
            .map_or(body.len(), |index| index);
        let (name, rest) = body.split_at(split);
        let name = std::str::from_utf8(name)
            .map_err(|_| scope.corrupt("an XML element name is not valid UTF-8"))?;
        if name.starts_with('!') || name.starts_with('?') {
            return Err(scope.corrupt("an XML declaration is malformed"));
        }
        let name = name.rsplit(':').next().map_or(name, |local| local);
        self.name.clear();
        self.name.push_str(name);
        self.attributes(trim(rest), scope)?;
        let Self {
            handler,
            name,
            attributes,
            closing,
            empty,
            ..
        } = self;
        if *closing {
            return handler.end(name.as_str());
        }
        let element = Element {
            name: name.as_str(),
            attributes,
        };
        handler.start(&element)?;
        if *empty {
            handler.end(name.as_str())?;
        }
        Ok(())
    }

    fn attributes(&mut self, mut rest: &[u8], scope: &Scope<'_>) -> ReportResult<()> {
        while !rest.is_empty() {
            let end = rest
                .iter()
                .position(|byte| byte.is_ascii_whitespace() || *byte == b'=')
                .map_or(rest.len(), |index| index);
            let (key, after) = rest.split_at(end);
            if key.is_empty() {
                return Err(scope.corrupt("an XML attribute has no name"));
            }
            let after = trim(after);
            let after = match after.strip_prefix(b"=") {
                Some(value) => trim(value),
                None => return Err(scope.corrupt("an XML attribute has no value")),
            };
            let (quote, value) = after
                .split_first()
                .ok_or_else(|| scope.corrupt("an XML attribute has no value"))?;
            let terminator = match *quote {
                b'"' => b'"',
                b'\'' => b'\'',
                _ => return Err(scope.corrupt("an XML attribute value is not quoted")),
            };
            let close = value
                .iter()
                .position(|byte| *byte == terminator)
                .ok_or_else(|| scope.corrupt("an XML attribute value is unterminated"))?;
            let (value, tail) = value.split_at(close);
            let key = std::str::from_utf8(key)
                .map_err(|_| scope.corrupt("an XML attribute name is not valid UTF-8"))?
                .to_string();
            let value = decode(value, scope)?;
            self.attributes.push((key, value));
            rest = trim(tail.get(1..).map_or(&[], |rest| rest));
        }
        Ok(())
    }
}

impl<H: Handler> Sink for Xml<'_, H> {
    fn push(&mut self, bytes: &[u8]) -> ReportResult<()> {
        self.feed(bytes)
    }
}
