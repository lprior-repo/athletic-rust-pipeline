use super::{active, attributes, bound, has_class, invalid, Element};
use census_store::{StoreError, StoreResult};
use html5gum::{DefaultEmitter, StartTag, Token, Tokenizer};

struct Pending {
    start: usize,
    attributes: Vec<(String, String)>,
}

#[derive(Default)]
struct Articles<'a> {
    stack: Vec<Option<Pending>>,
    count: usize,
    selected: Option<Element<'a>>,
}

pub(super) fn selected<'a>(html: &'a str, path: &str) -> StoreResult<Element<'a>> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut visibility = active::Active::default();
    let mut articles = Articles::default();
    Tokenizer::new_with_emitter(html, emitter)
        .take(100_001)
        .enumerate()
        .try_for_each(|(index, token)| -> StoreResult<()> {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                100_000,
                "participation article token count",
            )?;
            let token = token.map_err(|_| invalid("participation article input failed"))?;
            if !visibility.accept(&token)? {
                return Ok(());
            }
            match token {
                Token::StartTag(tag) if tag.name.as_slice() == b"article" => {
                    articles.start(&tag, path)
                }
                Token::EndTag(tag) if tag.name.as_slice() == b"article" => {
                    articles.close(html, tag.span.end)
                }
                _ => Ok(()),
            }
        })?;
    visibility.finish()?;
    if !articles.stack.is_empty() {
        return Err(invalid("unclosed participation article"));
    }
    articles
        .selected
        .ok_or_else(|| invalid("participation status article owner absent"))
}

impl<'a> Articles<'a> {
    fn start(&mut self, tag: &StartTag<usize>, path: &str) -> StoreResult<()> {
        self.count = self
            .count
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        bound(self.count, 512, "participation article count")?;
        bound(
            self.stack
                .len()
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?,
            128,
            "participation article depth",
        )?;
        if self.stack.iter().any(Option::is_some) {
            return Err(invalid("nested article inside selected status owner"));
        }
        let matches = has_class(tag, Some("node__article"))
            && tag
                .attributes
                .get(b"data-path".as_slice())
                .is_some_and(|value| value.as_slice() == path.as_bytes());
        if matches && self.selected.is_some() {
            return Err(invalid("participation status article owner is ambiguous"));
        }
        let pending = if matches {
            Some(Pending {
                start: tag.span.start,
                attributes: attributes(tag)?,
            })
        } else {
            None
        };
        self.stack
            .try_reserve(1)
            .map_err(|_| invalid("article stack allocation refused"))?;
        self.stack.push(pending);
        Ok(())
    }

    fn close(&mut self, html: &'a str, end: usize) -> StoreResult<()> {
        let pending = self
            .stack
            .pop()
            .ok_or_else(|| invalid("unmatched participation article close"))?;
        if let Some(pending) = pending {
            let span = pending.start..end;
            let body = html
                .get(span.clone())
                .ok_or_else(|| invalid("participation article span invalid"))?;
            self.selected = Some(Element {
                html: body,
                span,
                attributes: pending.attributes,
            });
        }
        Ok(())
    }
}
