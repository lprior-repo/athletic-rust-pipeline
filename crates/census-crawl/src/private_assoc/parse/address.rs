use regex::Regex;

#[derive(Default)]
pub struct AddressText {
    pub street: String,
    pub line2: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub plus4: String,
}

pub fn address_text(
    chunk: &str,
    div_pattern: &Regex,
    close_div_pattern: &Regex,
    br_pattern: &Regex,
    tag_pattern: &Regex,
) -> String {
    let found = div_pattern.captures_iter(chunk).find(|captures| {
        let classes = captures.get(1).map_or("", |matched| matched.as_str());
        has_class(classes, "address")
    });
    let Some(captures) = found else {
        return String::new();
    };
    let Some(tag) = captures.get(0) else {
        return String::new();
    };
    let rest = chunk
        .get(tag.end()..)
        .map_or(Default::default(), core::convert::identity);
    let inner = close_div_pattern
        .find(rest)
        .and_then(|close| rest.get(..close.start()))
        .map_or(rest, |value| value);
    tag_text(inner, br_pattern, tag_pattern)
}

pub(super) fn tag_text(fragment: &str, br_pattern: &Regex, tag_pattern: &Regex) -> String {
    let broken = br_pattern.replace_all(fragment, "\n");
    let stripped = tag_pattern.replace_all(broken.as_ref(), " ");
    collapse(decode_entities(stripped.as_ref()))
}

fn has_class(classes: &str, wanted: &str) -> bool {
    classes
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case(wanted))
}

pub fn split_address(text: &str) -> AddressText {
    let mut segments: Vec<&str> = text
        .split(['\n', ','])
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    let mut address = AddressText::default();
    if let Some((state, zip, plus4)) = segments.last().copied().and_then(state_zip) {
        address.state = state;
        address.zip = zip;
        address.plus4 = plus4;
        segments.pop();
    } else {
        if let Some((zip, plus4)) = segments.last().copied().and_then(zip_token) {
            address.zip = zip;
            address.plus4 = plus4;
            segments.pop();
        }
        if let Some(state) = segments.last().copied().and_then(state_token) {
            address.state = state;
            segments.pop();
        }
    }
    let city = segments
        .last()
        .copied()
        .filter(|segment| !segment.chars().any(|ch| ch.is_ascii_digit()));
    if let Some(city) = city {
        address.city = city.to_string();
        segments.pop();
    }
    if let Some((street, rest)) = segments.split_first() {
        address.street = (*street).to_string();
        address.line2 = rest.join(", ");
    }
    address
}

fn state_zip(token: &str) -> Option<(String, String, String)> {
    let mut words = token.split_whitespace();
    let (state, zip) = (words.next()?, words.next()?);
    if words.next().is_some() {
        return None;
    }
    let state = state_token(state)?;
    let (zip, plus4) = zip_token(zip)?;
    Some((state, zip, plus4))
}

fn zip_token(token: &str) -> Option<(String, String)> {
    let (code, plus4) = match token.split_once('-') {
        Some((code, plus4)) => (code, plus4),
        None => (token, ""),
    };
    let digits = |value: &str, limit: usize| {
        let count = value.chars().count();
        count >= 1 && count <= limit && value.chars().all(|ch| ch.is_ascii_digit())
    };
    let shaped = digits(code, 9) && (plus4.is_empty() || digits(plus4, 4));
    shaped.then(|| (code.to_string(), plus4.to_string()))
}

fn state_token(token: &str) -> Option<String> {
    (token.len() == 2 && token.chars().all(|ch| ch.is_ascii_alphabetic()))
        .then(|| token.to_string())
}

fn collapse(value: String) -> String {
    let mut out = String::with_capacity(value.len());
    let mut separator = false;
    for ch in value.chars() {
        if ch == '\n' {
            out.push('\n');
            separator = false;
        } else if ch.is_whitespace() {
            separator = !out.is_empty();
        } else {
            if separator {
                out.push(' ');
            }
            separator = false;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn decode_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(
            rest.get(..index)
                .map_or(Default::default(), core::convert::identity),
        );
        let tail = rest
            .get(index..)
            .map_or(Default::default(), core::convert::identity);
        let entity = tail.find(';').filter(|end| *end <= 12).and_then(|end| {
            let name = tail.get(1..end)?;
            let decoded = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                _ => name
                    .strip_prefix('#')
                    .and_then(|digits| digits.parse::<u32>().ok())
                    .and_then(char::from_u32),
            };
            decoded.map(|ch| (ch, end.saturating_add(1)))
        });
        match entity {
            Some((ch, consumed)) => {
                out.push(ch);
                rest = tail
                    .get(consumed..)
                    .map_or(Default::default(), core::convert::identity);
            }
            None => {
                out.push('&');
                rest = tail
                    .get(1..)
                    .map_or(Default::default(), core::convert::identity);
            }
        }
    }
    out.push_str(rest);
    out
}
