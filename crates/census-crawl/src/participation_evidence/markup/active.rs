use super::{bound, invalid};
use census_store::{StoreError, StoreResult};
use html5gum::{StartTag, Token};

#[derive(Default)]
pub(super) struct Active {
    excluded: Vec<(Vec<u8>, bool)>,
}

impl Active {
    pub(super) fn accept<S: html5gum::SpanBound>(&mut self, token: &Token<S>) -> StoreResult<bool> {
        match token {
            Token::StartTag(tag) => {
                let excluded = !self.excluded.is_empty() || forbidden(tag);
                if !excluded {
                    return Ok(true);
                }
                let foreign = matches!(tag.name.as_slice(), b"svg" | b"math")
                    || self.excluded.last().is_some_and(|(_, foreign)| *foreign);
                if !void(tag.name.as_slice()) && !(foreign && tag.self_closing) {
                    bound(
                        self.excluded
                            .len()
                            .checked_add(1)
                            .ok_or(StoreError::CounterOverflow)?,
                        128,
                        "participation inactive ancestor depth",
                    )?;
                    bound(tag.name.len(), 64, "participation inactive tag bytes")?;
                    self.excluded
                        .try_reserve(1)
                        .map_err(|_| invalid("inactive ancestor allocation refused"))?;
                    self.excluded.push((tag.name.as_slice().to_vec(), foreign));
                }
                Ok(false)
            }
            Token::EndTag(tag) if !self.excluded.is_empty() => {
                if self
                    .excluded
                    .last()
                    .is_none_or(|(name, _)| name.as_slice() != tag.name.as_slice())
                {
                    return Err(invalid("ambiguous inactive ancestor nesting"));
                }
                self.excluded.pop();
                Ok(false)
            }
            Token::EndTag(tag) if container(tag.name.as_slice()) => {
                Err(invalid("unmatched inactive container close"))
            }
            _ => Ok(self.excluded.is_empty()),
        }
    }

    pub(super) fn finish(self) -> StoreResult<()> {
        if self.excluded.is_empty() {
            Ok(())
        } else {
            Err(invalid("unclosed inactive ancestor"))
        }
    }
}

pub(super) fn forbidden<S: html5gum::SpanBound>(tag: &StartTag<S>) -> bool {
    container(tag.name.as_slice())
        || tag.attributes.contains_key(b"hidden".as_slice())
        || tag.attributes.contains_key(b"inert".as_slice())
}

fn container(name: &[u8]) -> bool {
    matches!(
        name,
        b"template" | b"svg" | b"math" | b"noscript" | b"iframe"
    )
}

fn void(name: &[u8]) -> bool {
    matches!(
        name,
        b"area"
            | b"base"
            | b"br"
            | b"col"
            | b"embed"
            | b"hr"
            | b"img"
            | b"input"
            | b"link"
            | b"meta"
            | b"param"
            | b"source"
            | b"track"
            | b"wbr"
    )
}
