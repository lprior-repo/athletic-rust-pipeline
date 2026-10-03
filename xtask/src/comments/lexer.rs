use super::literals;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Whitespace,
    LineComment,
    BlockComment { terminated: bool },
    Literal { terminated: bool },
    Pound,
    Bang,
    OpenBracket,
    CloseBracket,
    Eq,
    Ident,
    RawIdent,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Token {
    pub(super) kind: Kind,
    pub(super) len: usize,
}

pub(super) fn tokens(source: &str) -> Tokenizer<'_> {
    Tokenizer { rest: source }
}

pub(super) struct Tokenizer<'a> {
    rest: &'a str,
}

impl Iterator for Tokenizer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        let rest = self.rest;
        let first = rest.chars().next()?;
        let (kind, tail) = classify(rest, first);
        self.rest = tail;
        Some(Token {
            kind,
            len: rest.len().saturating_sub(tail.len()),
        })
    }
}

pub(super) fn after(rest: &str, bytes: usize) -> &str {
    rest.get(bytes..).map_or(Default::default(), core::convert::identity)
}

pub(super) fn take_while(rest: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut end = start;
    for (index, character) in after(rest, start).char_indices() {
        if !predicate(character) {
            break;
        }
        end = start
            .saturating_add(index)
            .saturating_add(character.len_utf8());
    }
    end
}

pub(super) fn is_ident_start(character: char) -> bool {
    character == '_' || unicode_ident::is_xid_start(character)
}

pub(super) fn is_ident_continue(character: char) -> bool {
    unicode_ident::is_xid_continue(character)
}

pub(super) fn is_lexical_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000a}'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{000d}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

fn classify(rest: &str, first: char) -> (Kind, &str) {
    match first {
        '\'' => literals::quote(rest),
        '"' => literals::string(rest, 0),
        '/' if rest.starts_with("//") => (Kind::LineComment, line_comment(rest)),
        '/' if rest.starts_with("/*") => literals::block_comment(rest),
        '#' => (Kind::Pound, after(rest, 1)),
        '!' => (Kind::Bang, after(rest, 1)),
        '[' => (Kind::OpenBracket, after(rest, 1)),
        ']' => (Kind::CloseBracket, after(rest, 1)),
        '=' => (Kind::Eq, after(rest, 1)),
        character if is_lexical_whitespace(character) => (Kind::Whitespace, whitespace(rest)),
        character if is_ident_start(character) => literals::ident(rest),
        _ => (Kind::Other, after(rest, first.len_utf8())),
    }
}

fn whitespace(rest: &str) -> &str {
    let end = rest
        .char_indices()
        .find(|(_, character)| !is_lexical_whitespace(*character))
        .map_or(rest.len(), |(index, _)| index);
    after(rest, end)
}

fn line_comment(rest: &str) -> &str {
    rest.find('\n').map_or("", |index| after(rest, index))
}
