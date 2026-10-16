use super::{Result, RetainedResultOwnershipError};
use html5gum::{DefaultEmitter, Token, Tokenizer};
use std::ops::Range;

pub(super) fn owned_pre(body: &str) -> Result<Range<usize>> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut state = State::default();
    for (ordinal, token) in Tokenizer::new_with_emitter(body, emitter).enumerate() {
        if ordinal >= 100_000 {
            return Err(RetainedResultOwnershipError::Invalid(
                "result HTML token capacity",
            ));
        }
        state.accept(token.map_err(|_| invalid())?)?;
    }
    let span = state.complete.ok_or_else(invalid)?;
    if state.depth != 0
        || state.open.is_some()
        || body.find("<pre") != Some(span.0)
        || body.get(span.0..).and_then(|tail| {
            tail.find('>')
                .and_then(|offset| offset.checked_add(span.0)?.checked_add(1))
        }) != Some(span.1)
        || body.get(span.1..).and_then(|tail| {
            tail.find("</pre>")
                .and_then(|offset| offset.checked_add(span.1))
        }) != Some(span.2)
    {
        return Err(invalid());
    }
    Ok(span.1..span.2)
}

#[derive(Default)]
struct State {
    inert: [Option<Container>; 32],
    depth: usize,
    open: Option<(usize, usize)>,
    complete: Option<(usize, usize, usize)>,
}

impl State {
    fn accept(&mut self, token: Token<usize>) -> Result<()> {
        match token {
            Token::StartTag(tag) => {
                if tag.attributes.contains_key(b"hidden".as_slice())
                    || tag.attributes.contains_key(b"inert".as_slice())
                {
                    return Err(invalid());
                }
                if let Some(container) = Container::of(tag.name.as_slice()) {
                    let entry = self.inert.get_mut(self.depth).ok_or_else(invalid)?;
                    *entry = Some(container);
                    self.depth = self
                        .depth
                        .checked_add(1)
                        .ok_or(RetainedResultOwnershipError::Overflow)?;
                }
                if tag.name.as_slice() == b"pre" {
                    if self.depth != 0 || self.open.is_some() || self.complete.is_some() {
                        return Err(invalid());
                    }
                    self.open = Some((tag.span.start, tag.span.end));
                }
            }
            Token::EndTag(tag) => {
                if tag.name.as_slice() == b"pre" {
                    let open = self.open.take().ok_or_else(invalid)?;
                    self.complete = Some((open.0, open.1, tag.span.start));
                }
                if let Some(container) = Container::of(tag.name.as_slice()) {
                    self.depth = self.depth.checked_sub(1).ok_or_else(invalid)?;
                    let entry = self.inert.get_mut(self.depth).ok_or_else(invalid)?;
                    if *entry != Some(container) {
                        return Err(invalid());
                    }
                    *entry = None;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Container {
    Template,
    Svg,
    Math,
    Noscript,
}

impl Container {
    fn of(tag: &[u8]) -> Option<Self> {
        match tag {
            b"template" => Some(Self::Template),
            b"svg" => Some(Self::Svg),
            b"math" => Some(Self::Math),
            b"noscript" => Some(Self::Noscript),
            _ => None,
        }
    }
}

fn invalid() -> RetainedResultOwnershipError {
    RetainedResultOwnershipError::Invalid(
        "result block is not one exact active original pre element",
    )
}
