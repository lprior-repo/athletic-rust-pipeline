use census_domain::school_directory::DirectoryError;
use regex::Regex;

mod text;
use text::{collapse, decode_entities};

#[derive(Default)]
pub struct AddressText<'a> {
    pub street: &'a str,
    pub line2: String,
    pub city: &'a str,
    pub state: &'a str,
    pub zip: &'a str,
    pub plus4: &'a str,
}

pub fn address_text(
    chunk: &str,
    div: &Regex,
    close: &Regex,
    br: &Regex,
    tag: &Regex,
) -> Result<String, DirectoryError> {
    let Some(open) = address_open(chunk, div) else {
        return Ok(String::new());
    };
    tag_text(address_inner(chunk, open, close)?, br, tag)
}

fn address_open<'a>(chunk: &'a str, div: &Regex) -> Option<regex::Match<'a>> {
    div.captures_iter(chunk)
        .find(is_address)
        .and_then(|captures| captures.get(0))
}

fn is_address(captures: &regex::Captures<'_>) -> bool {
    let classes = captures.get(1).map_or("", |value| value.as_str());
    super::has_class(classes, "address")
}

fn address_inner<'a>(
    chunk: &'a str,
    open: regex::Match<'_>,
    close: &Regex,
) -> Result<&'a str, DirectoryError> {
    let rest = chunk.get(open.end()..).ok_or_else(locator_error)?;
    match close.find(rest) {
        Some(end) => rest.get(..end.start()).ok_or_else(locator_error),
        None => Ok(rest),
    }
}

pub(super) fn tag_text(fragment: &str, br: &Regex, tag: &Regex) -> Result<String, DirectoryError> {
    let mut stripped = buffer(fragment.len())?;
    let end = tag.find_iter(fragment).try_fold(0, |start, matched| {
        let plain = fragment
            .get(start..matched.start())
            .ok_or_else(locator_error)?;
        stripped.push_str(plain);
        stripped.push(if br.is_match(matched.as_str()) {
            '\n'
        } else {
            ' '
        });
        Ok::<_, DirectoryError>(matched.end())
    })?;
    stripped.push_str(fragment.get(end..).ok_or_else(locator_error)?);
    collapse(&decode_entities(&stripped)?)
}

pub fn split_address(text: &str) -> Result<AddressText<'_>, DirectoryError> {
    let mut segments = segments(text)?;
    let mut address = AddressText::default();
    read_tail(&mut segments, &mut address);
    if let Some(city) = segments.last().copied().filter(is_city) {
        address.city = city;
        segments.truncate(segments.len().saturating_sub(1));
    }
    if let Some((street, rest)) = segments.split_first() {
        address.street = street;
        address.line2 = join_lines(rest)?;
    }
    Ok(address)
}

fn segments(text: &str) -> Result<Vec<&str>, DirectoryError> {
    let mut segments = Vec::new();
    text.split(['\n', ','])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .try_for_each(|value| {
            super::check_size(
                "association address segments",
                segments.len().saturating_add(1),
                128,
            )?;
            segments
                .try_reserve(1)
                .map_err(|_| DirectoryError::Allocation {
                    resource: "association address segments",
                })?;
            segments.push(value);
            Ok::<_, DirectoryError>(())
        })?;
    Ok(segments)
}

fn is_city(value: &&str) -> bool {
    !value.chars().any(|ch| ch.is_ascii_digit())
}

fn join_lines(parts: &[&str]) -> Result<String, DirectoryError> {
    let bytes = parts.iter().try_fold(0usize, |total, value| {
        total
            .checked_add(value.len())
            .and_then(|size| size.checked_add(2))
            .ok_or(DirectoryError::Capacity {
                resource: "association address bytes",
                requested: usize::MAX,
                limit: 16 * 1024,
            })
    })?;
    let mut text = buffer(bytes)?;
    parts.iter().enumerate().for_each(|(index, value)| {
        if index != 0 {
            text.push_str(", ");
        }
        text.push_str(value);
    });
    Ok(text)
}

fn read_tail<'a>(segments: &mut Vec<&'a str>, address: &mut AddressText<'a>) {
    if let Some((state, zip, plus4)) = segments.last().copied().and_then(state_zip) {
        address.state = state;
        address.zip = zip;
        address.plus4 = plus4;
        segments.truncate(segments.len().saturating_sub(1));
    } else {
        read_single_tail(segments, address);
    }
}

fn read_single_tail<'a>(segments: &mut Vec<&'a str>, address: &mut AddressText<'a>) {
    if let Some((zip, plus4)) = segments.last().copied().and_then(zip_token) {
        address.zip = zip;
        address.plus4 = plus4;
        segments.truncate(segments.len().saturating_sub(1));
    }
    if let Some(state) = segments.last().copied().and_then(state_token) {
        address.state = state;
        segments.truncate(segments.len().saturating_sub(1));
    }
}

fn state_zip(token: &str) -> Option<(&str, &str, &str)> {
    let mut words = token.split_whitespace();
    let (state, zip) = (words.next()?, words.next()?);
    if words.next().is_some() {
        return None;
    }
    let state = state_token(state)?;
    let (zip, plus4) = zip_token(zip)?;
    Some((state, zip, plus4))
}

fn zip_token(token: &str) -> Option<(&str, &str)> {
    let (code, plus4) = match token.split_once('-') {
        Some(parts) => parts,
        None => (token, ""),
    };
    (digits(code, 9) && (plus4.is_empty() || digits(plus4, 4))).then_some((code, plus4))
}

fn digits(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && value.bytes().all(|ch| ch.is_ascii_digit())
}

fn state_token(token: &str) -> Option<&str> {
    (token.len() == 2 && token.bytes().all(|ch| ch.is_ascii_alphabetic())).then_some(token)
}

fn buffer(bytes: usize) -> Result<String, DirectoryError> {
    super::check_size("association text bytes", bytes, 16 * 1024)?;
    let mut text = String::new();
    text.try_reserve_exact(bytes)
        .map_err(|_| DirectoryError::Allocation {
            resource: "association text",
        })?;
    Ok(text)
}

fn locator_error() -> DirectoryError {
    DirectoryError::Representation {
        detail: "association markup has an invalid locator".to_string(),
    }
}
