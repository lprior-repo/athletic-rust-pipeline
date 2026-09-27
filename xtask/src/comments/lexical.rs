use anyhow::{ensure, Context, Result};
use ra_ap_rustc_lexer::{tokenize, FrontmatterAllowed, LiteralKind, TokenKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Violation {
    pub(super) offset: usize,
    pub(super) reason: &'static str,
}

pub(super) fn first_violation(source: &str) -> Result<Option<Violation>> {
    let mut offset = 0usize;
    let mut attribute = Attribute::default();
    for token in tokenize(source, FrontmatterAllowed::No) {
        let end = offset.checked_add(usize::try_from(token.len)?)
            .context("Rust token position overflow")?;
        let text = source.get(offset..end).context("invalid Rust token span")?;
        if matches!(token.kind, TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }) {
            return Ok(Some(Violation { offset, reason: "code comments are forbidden" }));
        }
        if let TokenKind::Literal { kind, .. } = token.kind {
            ensure!(literal_terminated(kind), "unterminated Rust literal at byte {offset}");
        }
        if let Some(offset) = attribute.advance(token.kind, text, offset)? {
            return Ok(Some(Violation { offset, reason: "documentation attributes are forbidden" }));
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
    fn advance(&mut self, kind: TokenKind, text: &str, offset: usize) -> Result<Option<usize>> {
        if kind == TokenKind::Whitespace {
            return Ok(None);
        }
        if let Some(doc) = self.doc.take() {
            if kind == TokenKind::Eq {
                return Ok(Some(doc));
            }
        }
        match kind {
            TokenKind::Pound => self.prefix = true,
            TokenKind::Bang if self.prefix => {}
            TokenKind::OpenBracket if self.prefix || self.brackets > 0 => {
                self.brackets = self.brackets.checked_add(1).context("attribute nesting overflow")?;
                self.prefix = false;
            }
            TokenKind::CloseBracket => {
                self.brackets = self.brackets.saturating_sub(1);
                self.prefix = false;
            }
            TokenKind::Ident | TokenKind::RawIdent
                if self.brackets > 0 && matches!(text, "doc" | "r#doc") => {
                self.doc = Some(offset);
                self.prefix = false;
            }
            _ => self.prefix = false,
        }
        Ok(None)
    }
}

fn literal_terminated(kind: LiteralKind) -> bool {
    match kind {
        LiteralKind::Char { terminated }
        | LiteralKind::Byte { terminated }
        | LiteralKind::Str { terminated }
        | LiteralKind::ByteStr { terminated }
        | LiteralKind::CStr { terminated } => terminated,
        LiteralKind::RawStr { n_hashes }
        | LiteralKind::RawByteStr { n_hashes }
        | LiteralKind::RawCStr { n_hashes } => n_hashes.is_some(),
        LiteralKind::Int { .. } | LiteralKind::Float { .. } => true,
    }
}
