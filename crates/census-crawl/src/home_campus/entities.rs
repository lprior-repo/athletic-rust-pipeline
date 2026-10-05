pub(super) fn decode_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some((head, tail)) = rest.split_once('&') {
        out.push_str(head);
        let decoded = tail
            .split_once(';')
            .and_then(|(entity, after)| decode_entity(entity).map(|value| (value, after)));
        match decoded {
            Some((value, after)) => {
                out.push_str(&value);
                rest = after;
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

fn decode_entity(entity: &str) -> Option<String> {
    let named = match entity {
        "amp" => "&",
        "quot" => "\"",
        "apos" => "'",
        "lt" => "<",
        "gt" => ">",
        "nbsp" => " ",
        _ => {
            let digits = entity.strip_prefix('#')?;
            let code = match digits
                .strip_prefix('x')
                .or_else(|| digits.strip_prefix('X'))
            {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse::<u32>().ok()?,
            };
            return char::from_u32(code).map(String::from);
        }
    };
    Some(named.to_string())
}
