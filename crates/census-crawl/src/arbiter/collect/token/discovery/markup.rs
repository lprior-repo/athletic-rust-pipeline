use fields::{Field, Name};
use html5gum::State;

mod emit;
mod fields;
mod scopes;

pub(super) enum Declaration {
    Base(Vec<u8>),
    Module(Vec<u8>),
    Tick,
    Invalid(&'static str),
}

#[derive(Default)]
pub(super) struct Markup {
    tag: Name,
    last_start_tag: Name,
    closing: bool,
    in_tag: bool,
    self_closing: bool,
    attribute: Name,
    href: Field,
    kind: Field,
    src: Field,
    scopes: scopes::Scopes,
    scripts: usize,
    bases: usize,
    script_open: bool,
    queued: Option<Declaration>,
}

impl Markup {
    fn field(&mut self) -> Option<&mut Field> {
        if self.closing || self.scopes.is_inert() {
            return None;
        }
        match (self.tag.bytes(), self.attribute.bytes()) {
            (b"script", b"type") => Some(&mut self.kind),
            (b"script", b"src") => Some(&mut self.src),
            (b"base", b"href") => Some(&mut self.href),
            _ => None,
        }
    }

    fn finish_attribute(&mut self) {
        if let Some(field) = self.field() {
            field.finish();
        }
    }

    fn reset_tag(&mut self, closing: bool) {
        self.tag.clear();
        self.attribute.clear();
        self.href.clear();
        self.kind.clear();
        self.src.clear();
        self.closing = closing;
        self.self_closing = false;
        self.in_tag = true;
    }

    fn inert(&mut self) -> Result<bool, &'static str> {
        self.scopes
            .observe(self.tag.bytes(), self.closing, self.self_closing)
    }

    fn declaration(&mut self) -> Declaration {
        if self.tag.bytes() == b"script" {
            self.script_open = !self.closing && !self.scopes.is_foreign();
        }
        match self.inert() {
            Ok(true) => return Declaration::Tick,
            Err(reason) => return Declaration::Invalid(reason),
            Ok(false) => {}
        }
        if self.closing {
            return Declaration::Tick;
        }
        match self.tag.bytes() {
            b"script" => self.script_declaration(),
            b"base" => {
                self.bases = self.bases.saturating_add(1);
                if self.bases > 32 {
                    return Declaration::Invalid("directory HTML exceeds 32 bases");
                }
                if self.href.invalid() {
                    return Declaration::Invalid(
                        "directory URL exceeds 4096 bytes or allocation failed",
                    );
                }
                match self.href.take() {
                    Some(href) => Declaration::Base(href),
                    None => Declaration::Tick,
                }
            }
            _ => Declaration::Tick,
        }
    }

    fn script_declaration(&mut self) -> Declaration {
        self.scripts = self.scripts.saturating_add(1);
        if self.scripts > 128 {
            return Declaration::Invalid("directory HTML exceeds 128 scripts");
        }
        if self.kind.invalid() || self.src.invalid() {
            return Declaration::Invalid(
                "directory script attribute exceeds 4096 bytes or allocation failed",
            );
        }
        let module = self
            .kind
            .value()
            .is_some_and(|value| value.trim_ascii().eq_ignore_ascii_case(b"module"));
        if !module {
            return Declaration::Tick;
        }
        match self.src.take() {
            Some(src) => Declaration::Module(src),
            None => Declaration::Tick,
        }
    }

    fn next_state(&mut self) -> Option<State> {
        if self.closing {
            return None;
        }
        std::mem::swap(&mut self.tag, &mut self.last_start_tag);
        if self.scopes.is_foreign() {
            return None;
        }
        match self.last_start_tag.bytes() {
            b"noscript" => Some(State::RawText),
            name => html5gum::naive_next_state(name),
        }
    }
}
