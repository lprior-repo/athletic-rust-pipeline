use super::{Page, ParseResult};
use html5gum::{Error, Spanned, Token};

impl Page {
    pub(super) fn observe(&mut self, body: &[u8], token: Token<usize>) -> ParseResult<()> {
        match token {
            Token::StartTag(tag) => self.start(tag),
            Token::EndTag(tag) => self.end(&tag.name),
            Token::String(text) if self.inert.is_empty() => self.text(&text),
            Token::Error(error) if presentation_duplicate(body, &error) => Ok(()),
            Token::Error(error) => {
                self.malformed(&format!("school contact HTML tokenizer error: {error:?}"));
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

fn presentation_duplicate(body: &[u8], error: &Spanned<Error, usize>) -> bool {
    if error.value != Error::DuplicateAttribute {
        return false;
    }
    let Some(name) = body.get(error.span.start..error.span.end) else {
        return false;
    };
    let Some(suffix) = name.strip_prefix(b"data-v-") else {
        return false;
    };
    suffix.len() == 8
        && suffix
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests;
