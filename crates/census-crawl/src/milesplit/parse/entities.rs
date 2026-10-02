const MAX_ENTITY_DIGITS: usize = 7;

pub(in crate::milesplit) fn html_unescape(value: &str) -> String {
    let Some((head, tail)) = value.split_once('&') else {
        return value.to_owned();
    };
    let mut decoded = String::with_capacity(value.len());
    decoded.push_str(head);
    tail.split('&').fold(decoded, |mut output, fragment| {
        match numeric_entity(fragment).or_else(|| named_entity(fragment)) {
            Some((character, consumed)) => {
                output.push(character);
                output.push_str(fragment.get(consumed..).map_or("", |remaining| remaining));
            }
            None => {
                output.push('&');
                output.push_str(fragment);
            }
        }
        output
    })
}

fn named_entity(fragment: &str) -> Option<(char, usize)> {
    [
        ("amp;", '&'),
        ("apos;", '\''),
        ("quot;", '"'),
        ("lt;", '<'),
        ("gt;", '>'),
        ("nbsp;", ' '),
    ]
    .into_iter()
    .find_map(|(entity, character)| {
        fragment
            .starts_with(entity)
            .then_some((character, entity.len()))
    })
}

fn numeric_entity(tail: &str) -> Option<(char, usize)> {
    let body = tail.strip_prefix('#')?;
    let (digits, radix, marker): (&str, u32, usize) = match body.strip_prefix(['x', 'X']) {
        Some(hex) => (hex, 16, 2),
        None => (body, 10, 1),
    };
    let end = digits
        .chars()
        .take(MAX_ENTITY_DIGITS.saturating_add(1))
        .take_while(|character| character.is_digit(radix))
        .count();
    let terminated = end > 0
        && end <= MAX_ENTITY_DIGITS
        && digits.get(end..).is_some_and(|rest| rest.starts_with(';'));
    if !terminated {
        return None;
    }
    let code = u32::from_str_radix(digits.get(..end)?, radix).ok()?;
    let character = char::from_u32(code)?;
    Some((character, marker.saturating_add(end).saturating_add(1)))
}
