pub(super) fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some((head, tail)) = rest.split_once('&') {
        out.push_str(head);
        match tail.split_once(';') {
            Some((entity, after)) if entity.len() <= 10 && !entity.is_empty() => {
                match decode(entity) {
                    Some(character) => out.push(character),
                    None => {
                        out.push('&');
                        out.push_str(entity);
                        out.push(';');
                    }
                }
                rest = after;
            }
            _ => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" | "#x27" | "#X27" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => {
            let digits = entity.strip_prefix('#')?;
            let (radix, digits) = match digits.strip_prefix(['x', 'X']) {
                Some(hex) => (16, hex),
                None => (10, digits),
            };
            u32::from_str_radix(digits, radix)
                .ok()
                .and_then(char::from_u32)
        }
    }
}
