use fields::{Attribute, Fields, Name};
use html5gum::State;

mod emit;
mod fields;

pub(super) enum Declaration {
    Url(Vec<u8>),
    Script(Vec<u8>),
    Invalid,
}

#[derive(Default)]
pub(super) struct Markup {
    tag: Name,
    last_start_tag: Name,
    closing: bool,
    attribute: Name,
    fields: Fields,
    template_depth: usize,
    foreign_depth: usize,
    script: bool,
    body: Vec<u8>,
    queued: Option<Declaration>,
}

impl Markup {
    fn selected_attribute(&self) -> Option<Attribute> {
        if self.closing || self.template_depth != 0 || self.foreign_depth != 0 {
            return None;
        }
        let attribute = Attribute::parse(self.attribute.bytes())?;
        match (self.tag.bytes(), attribute) {
            (b"link", Attribute::Rel | Attribute::Href)
            | (b"meta", Attribute::Property | Attribute::Name | Attribute::Content)
            | (b"script", Attribute::Type) => Some(attribute),
            _ => None,
        }
    }

    fn finish_attribute(&mut self) {
        if let Some(attribute) = self.selected_attribute() {
            self.fields.finish(attribute);
        }
    }

    fn reset_tag(&mut self, closing: bool) {
        self.tag.clear();
        self.attribute.clear();
        self.fields.clear();
        self.closing = closing;
    }

    fn inert(&mut self) -> bool {
        let depth = match self.tag.bytes() {
            b"template" => Some(&mut self.template_depth),
            b"svg" | b"math" => Some(&mut self.foreign_depth),
            _ => None,
        };
        if let Some(depth) = depth {
            if self.closing {
                *depth = depth.saturating_sub(1);
            } else if let Some(next) = depth.checked_add(1) {
                *depth = next;
            } else {
                self.queued = Some(Declaration::Invalid);
            }
            return true;
        }
        self.template_depth != 0 || self.foreign_depth != 0
    }

    fn emit_declaration(&mut self) {
        if self.closing && self.tag.bytes() == b"script" {
            if self.script {
                self.queued = Some(Declaration::Script(std::mem::take(&mut self.body)));
            }
            self.script = false;
        }
        if self.inert() || self.closing {
            return;
        }
        let selected = match self.tag.bytes() {
            b"link" => self.fields.value(Attribute::Rel).and_then(|rel| {
                rel.split(u8::is_ascii_whitespace)
                    .any(|token| token.eq_ignore_ascii_case(b"canonical"))
                    .then_some(Attribute::Href)
            }),
            b"meta" => self
                .fields
                .value(Attribute::Property)
                .or_else(|| self.fields.value(Attribute::Name))
                .filter(|value| {
                    value.eq_ignore_ascii_case(b"og:url")
                        || value.eq_ignore_ascii_case(b"twitter:url")
                })
                .map(|_| Attribute::Content),
            b"script" => {
                self.script = self
                    .fields
                    .value(Attribute::Type)
                    .is_some_and(|kind| kind.eq_ignore_ascii_case(b"application/ld+json"));
                None
            }
            _ => None,
        };
        if let Some(bytes) = selected.and_then(|attribute| self.fields.take(attribute)) {
            self.queued = Some(Declaration::Url(bytes));
        }
    }

    fn next_state(&mut self) -> Option<State> {
        if self.closing {
            return None;
        }
        std::mem::swap(&mut self.tag, &mut self.last_start_tag);
        match self.last_start_tag.bytes() {
            b"noscript" => Some(State::RawText),
            name => html5gum::naive_next_state(name),
        }
    }
}
