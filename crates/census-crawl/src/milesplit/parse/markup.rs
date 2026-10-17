use fields::{Attribute, Fields, Name};
use html5gum::State;
use inert::{Container, Containers};

mod emit;
mod fields;
mod inert;
mod location;
use location::Position;
pub(in crate::milesplit) use location::{DeclarationKind, LocatedDeclaration, Termination};

pub(in crate::milesplit) enum Declaration {
    Url(Vec<u8>),
    Script(Vec<u8>),
    Invalid,
}

#[derive(Default)]
pub(in crate::milesplit) struct Markup {
    tag: Name,
    last_start_tag: Name,
    closing: bool,
    self_closing: bool,
    attribute: Name,
    fields: Fields,
    containers: Containers,
    script: bool,
    body: Vec<u8>,
    queued: Option<LocatedDeclaration>,
    position: Position,
}

impl Markup {
    fn selected_attribute(&self) -> Option<Attribute> {
        if self.closing || !self.containers.is_empty() {
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
        self.self_closing = false;
    }

    fn inert(&mut self) -> bool {
        let Some(container) = Container::from_tag(self.tag.bytes()) else {
            return !self.containers.is_empty();
        };
        if !self.closing && self.self_closing && container != Container::Template {
            return true;
        }
        let valid = if self.closing {
            self.containers.close(container)
        } else {
            self.containers.open(container)
        };
        if !valid {
            self.queue(
                Declaration::Invalid,
                DeclarationKind::Invalid,
                Termination::Tag,
            );
        }
        true
    }

    fn emit_declaration(&mut self) {
        if self.closing && self.tag.bytes() == b"script" {
            if self.script {
                let body = std::mem::take(&mut self.body);
                self.queue(
                    Declaration::Script(body),
                    DeclarationKind::JsonLd,
                    Termination::ScriptEnd,
                );
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
                    .then_some((Attribute::Href, DeclarationKind::Canonical))
            }),
            b"meta" => self
                .fields
                .value(Attribute::Property)
                .or_else(|| self.fields.value(Attribute::Name))
                .filter(|value| {
                    value.eq_ignore_ascii_case(b"og:url")
                        || value.eq_ignore_ascii_case(b"twitter:url")
                })
                .map(|value| {
                    (
                        Attribute::Content,
                        if value.eq_ignore_ascii_case(b"og:url") {
                            DeclarationKind::OpenGraph
                        } else {
                            DeclarationKind::Twitter
                        },
                    )
                }),
            b"script" => {
                self.script = self
                    .fields
                    .value(Attribute::Type)
                    .is_some_and(|kind| kind.eq_ignore_ascii_case(b"application/ld+json"));
                if self.script {
                    self.position.script();
                }
                None
            }
            _ => None,
        };
        if let Some((attribute, kind)) = selected {
            let declaration = match self.fields.take(attribute) {
                Some(bytes) => Declaration::Url(bytes),
                None => Declaration::Invalid,
            };
            self.queue(declaration, kind, Termination::Tag);
        }
    }

    fn queue(&mut self, declaration: Declaration, kind: DeclarationKind, termination: Termination) {
        self.queued = Some(self.position.locate(declaration, kind, termination));
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
