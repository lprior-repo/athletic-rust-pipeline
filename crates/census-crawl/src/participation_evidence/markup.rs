use super::{bound, invalid};
use census_store::{StoreError, StoreResult};
use html5gum::{DefaultEmitter, StartTag, Token, Tokenizer};
use std::ops::Range;

mod active;
mod article;

pub(super) fn status_article<'a>(html: &'a str, path: &str) -> StoreResult<Element<'a>> {
    article::selected(html, path)
}

type ElementAttributes = Vec<(String, String)>;

pub(super) struct Element<'a> {
    pub(super) html: &'a str,
    pub(super) span: Range<usize>,
    pub(super) attributes: Vec<(String, String)>,
}

impl Element<'_> {
    pub(super) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
    pub(super) fn locator(&self) -> String {
        format!("html_bytes:{}:{}", self.span.start, self.span.end)
    }
}

pub(super) fn elements<'a>(
    html: &'a str,
    name: &[u8],
    class: Option<&str>,
) -> StoreResult<Vec<Element<'a>>> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut active: Option<(usize, usize, ElementAttributes)> = None;
    let mut output = Vec::new();
    let mut visibility = active::Active::default();
    Tokenizer::new_with_emitter(html, emitter)
        .take(100_001)
        .enumerate()
        .try_for_each(|(index, token)| -> StoreResult<()> {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                100_000,
                "participation HTML token count",
            )?;
            let token = token.map_err(|_| invalid("participation HTML input failed"))?;
            if !visibility.accept(&token)? {
                return Ok(());
            }
            match token {
                Token::StartTag(tag) if tag.name.as_slice() == name => {
                    if let Some((_, depth, _)) = active.as_mut() {
                        *depth = depth.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                    } else if has_class(&tag, class) {
                        active = Some((tag.span.start, 1, attributes(&tag)?));
                    }
                }
                Token::EndTag(tag) if tag.name.as_slice() == name => {
                    if active.as_ref().is_some_and(|(_, depth, _)| *depth > 1) {
                        if let Some((_, depth, _)) = active.as_mut() {
                            *depth = depth.checked_sub(1).ok_or(StoreError::CounterOverflow)?;
                        }
                        return Ok(());
                    }
                    if let Some((start, _, attributes)) = active.take() {
                        bound(
                            output
                                .len()
                                .checked_add(1)
                                .ok_or(StoreError::CounterOverflow)?,
                            512,
                            "participation statement elements",
                        )?;
                        output
                            .try_reserve(1)
                            .map_err(|_| invalid("participation element allocation refused"))?;
                        let body = html
                            .get(start..tag.span.end)
                            .ok_or_else(|| invalid("participation statement span invalid"))?;
                        output.push(Element {
                            html: body,
                            span: start..tag.span.end,
                            attributes,
                        });
                    }
                }
                _ => {}
            }
            Ok(())
        })?;
    visibility.finish()?;
    if active.is_some() {
        return Err(invalid("unclosed participation statement element"));
    }
    Ok(output)
}

fn has_class(tag: &StartTag<usize>, class: Option<&str>) -> bool {
    class.is_none_or(|required| {
        tag.attributes
            .get(b"class".as_slice())
            .is_some_and(|value| {
                std::str::from_utf8(value).is_ok_and(|value| {
                    value
                        .split_ascii_whitespace()
                        .any(|class| class == required)
                })
            })
    })
}

fn attributes(tag: &StartTag<usize>) -> StoreResult<Vec<(String, String)>> {
    bound(tag.attributes.len(), 32, "participation element attributes")?;
    tag.attributes
        .iter()
        .map(|(key, value)| {
            bound(key.len(), 128, "participation attribute name bytes")?;
            bound(value.len(), 2048, "participation attribute value bytes")?;
            Ok((
                std::str::from_utf8(key)
                    .map_err(|_| invalid("participation attribute key encoding"))?
                    .into(),
                std::str::from_utf8(value)
                    .map_err(|_| invalid("participation attribute value encoding"))?
                    .into(),
            ))
        })
        .collect()
}

pub(super) fn text(html: &str) -> StoreResult<String> {
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    Tokenizer::new_with_emitter(html, emitter)
        .take(4097)
        .enumerate()
        .try_fold(String::new(), |mut text, (index, token)| {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                4096,
                "participation statement token count",
            )?;
            let token = token.map_err(|_| invalid("participation text input failed"))?;
            match token {
                Token::String(value) => append(
                    &mut text,
                    std::str::from_utf8(&value)
                        .map_err(|_| invalid("participation text encoding"))?,
                )?,
                Token::StartTag(tag)
                    if active::forbidden(&tag)
                        || matches!(tag.name.as_slice(), b"script" | b"style") =>
                {
                    return Err(invalid("inert markup inside participation statement"))
                }
                Token::StartTag(tag) if tag.name.as_slice() == b"br" => append(&mut text, " | ")?,
                _ => {}
            }
            Ok(text)
        })
        .map(|text| text.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn append(target: &mut String, value: &str) -> StoreResult<()> {
    let needed = target
        .len()
        .checked_add(value.len())
        .ok_or(StoreError::CounterOverflow)?;
    bound(needed, 4096, "participation statement text bytes")?;
    target
        .try_reserve(value.len())
        .map_err(|_| invalid("participation text allocation refused"))?;
    target.push_str(value);
    Ok(())
}

pub(super) fn single<'a>(mut values: Vec<Element<'a>>) -> StoreResult<Element<'a>> {
    if values.len() != 1 {
        return Err(invalid("participation statement is missing or ambiguous"));
    }
    values
        .pop()
        .ok_or_else(|| invalid("participation statement is missing"))
}

pub(super) fn canonical(html: &str, requested: &str) -> StoreResult<()> {
    let base = url::Url::parse(requested).map_err(|_| invalid("canonical source base invalid"))?;
    let mut emitter = DefaultEmitter::default();
    emitter.naively_switch_states(true);
    let mut found = false;
    let mut visibility = active::Active::default();
    Tokenizer::new_with_emitter(html, emitter)
        .take(100_001)
        .enumerate()
        .try_for_each(|(index, token)| {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                100_000,
                "participation canonical token count",
            )?;
            let token = token.map_err(|_| invalid("participation canonical input failed"))?;
            if !visibility.accept(&token)? {
                return Ok(());
            }
            match token {
                Token::StartTag(tag)
                    if tag.name.as_slice() == b"link" && canonical_relation(&tag)? =>
                {
                    let raw = tag
                        .attributes
                        .get(b"href".as_slice())
                        .ok_or_else(|| invalid("canonical URL absent"))?;
                    bound(raw.len(), 2048, "participation canonical URL bytes")?;
                    let raw =
                        std::str::from_utf8(raw).map_err(|_| invalid("canonical URL encoding"))?;
                    if base
                        .join(raw)
                        .map_err(|_| invalid("canonical URL invalid"))?
                        != base
                    {
                        return Err(invalid("participation declared canonical owner differs"));
                    }
                    found = true;
                }
                _ => {}
            }
            Ok(())
        })?;
    visibility.finish()?;
    if !found {
        return Err(invalid("participation canonical owner is missing"));
    }
    Ok(())
}

fn canonical_relation<S: html5gum::SpanBound>(tag: &StartTag<S>) -> StoreResult<bool> {
    let Some(rel) = tag.attributes.get(b"rel".as_slice()) else {
        return Ok(false);
    };
    bound(rel.len(), 2048, "participation canonical relation bytes")?;
    let rel = std::str::from_utf8(rel).map_err(|_| invalid("canonical relation encoding"))?;
    Ok(rel
        .split_ascii_whitespace()
        .any(|token| token.eq_ignore_ascii_case("canonical")))
}
