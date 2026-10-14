use super::date::{from_label, PublishedDate};
use super::html::collapse_whitespace;
use html5gum::{DefaultEmitter, HtmlString, StartTag, Token, Tokenizer};
use std::ops::ControlFlow;

const MAX_ROW_BYTES: usize = 65_536;
const MAX_TOKENS: usize = 8192;
const MAX_DEPTH: usize = 64;
const MAX_LABEL_BYTES: usize = 128;

enum Found {
    Missing,
    Complete(String),
}

enum Cell {
    Outside(Found),
    Ignored {
        name: HtmlString,
        depth: usize,
        found: Found,
    },
    Reading {
        stack: MarkupStack,
        label: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Markup {
    Div,
    Span,
    Bold,
    Strong,
    Italic,
    Emphasis,
    Small,
    Time,
    Anchor,
}

struct MarkupStack {
    entries: [Markup; MAX_DEPTH],
    length: usize,
}

type Step = ControlFlow<(), Cell>;

pub(super) fn published_date(row: &str) -> Option<PublishedDate> {
    if row.len() > MAX_ROW_BYTES {
        return None;
    }
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let outcome = Tokenizer::new_with_emitter(row, emitter)
        .take(MAX_TOKENS.saturating_add(1))
        .enumerate()
        .try_fold(Cell::Outside(Found::Missing), |cell, (index, token)| {
            if index >= MAX_TOKENS {
                return ControlFlow::Break(());
            }
            match token {
                Ok(Token::Error(_)) | Err(_) => ControlFlow::Break(()),
                Ok(token) => cell.absorb(token),
            }
        });
    match outcome {
        ControlFlow::Continue(Cell::Outside(Found::Complete(label))) => {
            from_label(collapse_whitespace(&label))
        }
        ControlFlow::Break(()) | ControlFlow::Continue(_) => None,
    }
}

impl Cell {
    fn absorb(self, token: Token) -> Step {
        match self {
            Self::Outside(found) => seek(found, token),
            Self::Ignored { name, depth, found } => ignored(name, depth, found, token),
            Self::Reading { stack, label } => reading(stack, label, token),
        }
    }
}

impl Markup {
    fn parse(name: &[u8]) -> Option<Self> {
        match name {
            b"div" => Some(Self::Div),
            b"span" => Some(Self::Span),
            b"b" => Some(Self::Bold),
            b"strong" => Some(Self::Strong),
            b"i" => Some(Self::Italic),
            b"em" => Some(Self::Emphasis),
            b"small" => Some(Self::Small),
            b"time" => Some(Self::Time),
            b"a" => Some(Self::Anchor),
            _ => None,
        }
    }

    fn separator(self) -> &'static str {
        match self {
            Self::Div => " ",
            _ => "",
        }
    }
}

impl MarkupStack {
    fn root() -> Self {
        Self {
            entries: [Markup::Div; MAX_DEPTH],
            length: 1,
        }
    }

    fn open(mut self, markup: Markup) -> Option<Self> {
        *self.entries.get_mut(self.length)? = markup;
        self.length = self.length.checked_add(1)?;
        Some(self)
    }

    fn close(mut self, markup: Markup) -> Option<Self> {
        let length = self.length.checked_sub(1)?;
        if self.entries.get(length) != Some(&markup) {
            return None;
        }
        self.length = length;
        Some(self)
    }
}

fn inert(name: &[u8]) -> bool {
    matches!(
        name,
        b"script"
            | b"style"
            | b"template"
            | b"svg"
            | b"math"
            | b"noscript"
            | b"textarea"
            | b"title"
            | b"iframe"
            | b"xmp"
            | b"noembed"
            | b"noframe"
            | b"noframes"
            | b"plaintext"
    )
}

fn seek(found: Found, token: Token) -> Step {
    let Token::StartTag(tag) = token else {
        return ControlFlow::Continue(Cell::Outside(found));
    };
    if inert(&tag.name) {
        return ignore_start(found, tag);
    }
    if !is_date(&tag) {
        return ControlFlow::Continue(Cell::Outside(found));
    }
    if tag.name.as_slice() != b"div" || tag.self_closing || matches!(found, Found::Complete(_)) {
        return ControlFlow::Break(());
    }
    let mut label = String::new();
    if label.try_reserve_exact(MAX_LABEL_BYTES).is_err() {
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(Cell::Reading {
        stack: MarkupStack::root(),
        label,
    })
}

fn ignore_start(found: Found, tag: StartTag<()>) -> Step {
    if tag.self_closing && matches!(tag.name.as_slice(), b"svg" | b"math") {
        return ControlFlow::Continue(Cell::Outside(found));
    }
    ControlFlow::Continue(Cell::Ignored {
        name: tag.name,
        depth: 1,
        found,
    })
}

fn is_date(tag: &StartTag<()>) -> bool {
    tag.attributes
        .get(b"data-label".as_slice())
        .is_some_and(|label| label.as_slice() == b"Meet Date")
}

fn ignored(name: HtmlString, depth: usize, found: Found, token: Token) -> Step {
    match token {
        Token::StartTag(tag) if tag.name == name => {
            if tag.self_closing && matches!(tag.name.as_slice(), b"svg" | b"math") {
                return ControlFlow::Continue(Cell::Ignored { name, depth, found });
            }
            let Some(next) = depth.checked_add(1).filter(|next| *next <= MAX_DEPTH) else {
                return ControlFlow::Break(());
            };
            ControlFlow::Continue(Cell::Ignored {
                name,
                depth: next,
                found,
            })
        }
        Token::EndTag(tag) if tag.name == name => match depth.checked_sub(1) {
            Some(0) => ControlFlow::Continue(Cell::Outside(found)),
            Some(next) => ControlFlow::Continue(Cell::Ignored {
                name,
                depth: next,
                found,
            }),
            None => ControlFlow::Break(()),
        },
        _ => ControlFlow::Continue(Cell::Ignored { name, depth, found }),
    }
}

fn reading(stack: MarkupStack, label: String, token: Token) -> Step {
    match token {
        Token::StartTag(tag) => nested(stack, label, tag),
        Token::EndTag(tag) => close(stack, label, &tag.name),
        Token::String(text) => match std::str::from_utf8(&text) {
            Ok(text) => append(stack, label, text),
            Err(_) => ControlFlow::Break(()),
        },
        Token::Comment(_) => ControlFlow::Continue(Cell::Reading { stack, label }),
        Token::Doctype(_) | Token::Error(_) => ControlFlow::Break(()),
    }
}

fn nested(stack: MarkupStack, label: String, tag: StartTag<()>) -> Step {
    if is_date(&tag) {
        return ControlFlow::Break(());
    }
    if tag.name.as_slice() == b"br" {
        return append(stack, label, " ");
    }
    let Some(markup) = Markup::parse(&tag.name).filter(|_| !tag.self_closing) else {
        return ControlFlow::Break(());
    };
    match stack.open(markup) {
        Some(stack) => append(stack, label, markup.separator()),
        None => ControlFlow::Break(()),
    }
}

fn close(stack: MarkupStack, label: String, name: &[u8]) -> Step {
    let Some(markup) = Markup::parse(name) else {
        return ControlFlow::Break(());
    };
    let Some(stack) = stack.close(markup) else {
        return ControlFlow::Break(());
    };
    if stack.length == 0 {
        return ControlFlow::Continue(Cell::Outside(Found::Complete(label)));
    }
    append(stack, label, markup.separator())
}

fn append(stack: MarkupStack, mut label: String, text: &str) -> Step {
    let Some(length) = label.len().checked_add(text.len()) else {
        return ControlFlow::Break(());
    };
    if length > MAX_LABEL_BYTES {
        return ControlFlow::Break(());
    }
    label.push_str(text);
    ControlFlow::Continue(Cell::Reading { stack, label })
}

#[cfg(test)]
#[path = "date_cell_tests.rs"]
mod tests;
