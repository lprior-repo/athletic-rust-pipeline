use super::lexer::{tokens, Kind, Token};
use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Violation {
    pub(super) offset: usize,
    pub(super) reason: &'static str,
}

pub(super) fn first_violation(source: &str) -> Result<Option<Violation>> {
    first_violation_over(tokens(source), source)
}

pub(super) fn first_violation_over(
    tokens: impl Iterator<Item = Token>,
    source: &str,
) -> Result<Option<Violation>> {
    let mut offset = 0usize;
    let mut attribute = Attribute::default();
    for token in tokens {
        let end = offset
            .checked_add(token.len)
            .context("Rust token position overflow")?;
        let text = source.get(offset..end).context("invalid Rust token span")?;
        match token.kind {
            Kind::LineComment | Kind::BlockComment { .. } => {
                return Ok(Some(Violation {
                    offset,
                    reason: "code comments are forbidden",
                }));
            }
            Kind::Literal { terminated: false } => {
                return Err(anyhow!("unterminated Rust literal at byte {offset}"));
            }
            _ => {}
        }
        if let Some(found) = attribute.advance(token.kind, text, offset) {
            return Ok(Some(Violation {
                offset: found,
                reason: "documentation attributes are forbidden",
            }));
        }
        offset = end;
    }
    Ok(None)
}

#[derive(Default)]
struct Attribute {
    brackets: usize,
    prefix: bool,
    doc: Option<usize>,
}

impl Attribute {
    fn advance(&mut self, kind: Kind, text: &str, offset: usize) -> Option<usize> {
        if kind == Kind::Whitespace {
            return None;
        }
        if let Some(doc) = self.doc.take() {
            if kind == Kind::Eq {
                return Some(doc);
            }
        }
        match kind {
            Kind::Pound => self.prefix = true,
            Kind::Bang if self.prefix => {}
            Kind::OpenBracket if self.prefix || self.brackets > 0 => {
                self.brackets = self.brackets.saturating_add(1);
                self.prefix = false;
            }
            Kind::CloseBracket => {
                self.brackets = self.brackets.saturating_sub(1);
                self.prefix = false;
            }
            Kind::Ident | Kind::RawIdent
                if self.brackets > 0 && matches!(text, "doc" | "r#doc") =>
            {
                self.doc = Some(offset);
                self.prefix = false;
            }
            _ => self.prefix = false,
        }
        None
    }
}
