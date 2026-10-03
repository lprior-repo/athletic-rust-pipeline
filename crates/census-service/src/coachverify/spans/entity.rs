const NAMED_ENTITIES: [(&str, char); 51] = [
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("copy", '\u{a9}'),
    ("reg", '\u{ae}'),
    ("trade", '\u{2122}'),
    ("hellip", '\u{2026}'),
    ("ndash", '\u{2013}'),
    ("mdash", '\u{2014}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201c}'),
    ("rdquo", '\u{201d}'),
    ("bull", '\u{2022}'),
    ("middot", '\u{b7}'),
    ("deg", '\u{b0}'),
    ("times", '\u{d7}'),
    ("divide", '\u{f7}'),
    ("frac12", '\u{bd}'),
    ("ordm", '\u{ba}'),
    ("sect", '\u{a7}'),
    ("para", '\u{b6}'),
    ("aacute", '\u{e1}'),
    ("eacute", '\u{e9}'),
    ("iacute", '\u{ed}'),
    ("oacute", '\u{f3}'),
    ("uacute", '\u{fa}'),
    ("agrave", '\u{e0}'),
    ("egrave", '\u{e8}'),
    ("ccedil", '\u{e7}'),
    ("ntilde", '\u{f1}'),
    ("auml", '\u{e4}'),
    ("ouml", '\u{f6}'),
    ("uuml", '\u{fc}'),
    ("szlig", '\u{df}'),
    ("euro", '\u{20ac}'),
    ("pound", '\u{a3}'),
    ("yen", '\u{a5}'),
    ("cent", '\u{a2}'),
    ("laquo", '\u{ab}'),
    ("raquo", '\u{bb}'),
    ("larr", '\u{2190}'),
    ("rarr", '\u{2192}'),
    ("harr", '\u{2194}'),
    ("le", '\u{2264}'),
    ("ge", '\u{2265}'),
    ("ne", '\u{2260}'),
    ("minus", '\u{2212}'),
];

pub(super) fn decode_entities(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(
            rest.get(..index)
                .map_or(Default::default(), core::convert::identity),
        );
        let candidate = rest
            .get(index..)
            .map_or(Default::default(), core::convert::identity);
        let tail = candidate
            .get(1..)
            .map_or(Default::default(), core::convert::identity);
        let Some(stop) = tail
            .get(..tail.len().min(64))
            .and_then(|head| head.find(';'))
        else {
            out.push('&');
            rest = tail;
            continue;
        };
        let body = tail
            .get(..stop)
            .map_or(Default::default(), core::convert::identity);
        match decode_entity(body) {
            Some(decoded) => {
                out.push(decoded);
                rest = tail
                    .get(stop.saturating_add(1)..)
                    .map_or(Default::default(), core::convert::identity);
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entity(body: &str) -> Option<char> {
    numeric_entity(body).or_else(|| named_entity(body))
}

fn numeric_entity(body: &str) -> Option<char> {
    if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
        return u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
    }
    let decimal = body.strip_prefix('#')?;
    decimal.parse::<u32>().ok().and_then(char::from_u32)
}

fn named_entity(body: &str) -> Option<char> {
    NAMED_ENTITIES
        .iter()
        .find(|(name, _)| *name == body)
        .map(|(_, decoded)| *decoded)
}
