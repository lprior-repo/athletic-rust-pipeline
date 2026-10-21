use super::{bounded, invalid, Result};
use html5gum::{StartTag, Token};

pub(super) fn hidden(tag: &StartTag<usize>) -> Result<bool> {
    if tag.attributes.contains_key(b"hidden".as_slice())
        || tag.attributes.contains_key(b"inert".as_slice())
    {
        return Ok(true);
    }
    if tag
        .attributes
        .get(b"aria-hidden".as_slice())
        .is_some_and(|value| value.as_slice().eq_ignore_ascii_case(b"true"))
    {
        return Ok(true);
    }
    let Some(style) = tag.attributes.get(b"style".as_slice()) else {
        return Ok(false);
    };
    bounded(style.len(), 4096)?;
    let style = std::str::from_utf8(style).map_err(|_| invalid("TFRRS style encoding"))?;
    let compact: String = style
        .chars()
        .filter(|value| !value.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    Ok(compact.split(';').any(|declaration| {
        declaration == "display:none"
            || declaration == "display:none!important"
            || declaration == "visibility:hidden"
            || declaration == "visibility:hidden!important"
    }))
}

pub(super) fn mark(token: &mut Token<usize>) -> Result<()> {
    if let Token::StartTag(tag) = token {
        if hidden(tag)? && !tag.attributes.contains_key(b"hidden".as_slice()) {
            tag.attributes
                .insert(b"hidden".to_vec().into(), Default::default());
        }
    }
    Ok(())
}
