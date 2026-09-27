pub fn normalize_name(raw: &str) -> String {
    let lowered = raw.trim().to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if ch.is_whitespace()
            || ch == '-'
            || ch == '\''
            || ch == '.'
            || ch == ','
            || ch == '/'
        {
            out.push(' ');
        } else if !ch.is_ascii() {
            if let Some(replacement) = strip_diacritic(ch) {
                out.push(replacement);
            }
        }
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut parts: Vec<&str> = collapsed.split(' ').collect();
    while strip_type_suffix(&mut parts) {}
    parts.join(" ")
}

fn strip_type_suffix(parts: &mut Vec<&str>) -> bool {
    for suffix in [
        "high school",
        "hs",
        "highschool",
        "school",
        "sr high",
        "senior high",
    ] {
        let tokens: Vec<&str> = suffix.split(' ').collect();
        if parts.len() > tokens.len() && parts.ends_with(&tokens) {
            parts.truncate(parts.len().saturating_sub(tokens.len()));
            return true;
        }
    }
    false
}

fn strip_diacritic(ch: char) -> Option<char> {
    let folded = match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'ē' | 'ę' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'ī' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ō' | 'ø' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'ū' => 'u',
        'ñ' | 'ń' => 'n',
        'ç' | 'ć' => 'c',
        'š' | 'ś' => 's',
        'ž' | 'ź' | 'ż' => 'z',
        'ý' | 'ÿ' => 'y',
        'ł' => 'l',
        'æ' => 'a',
        'œ' => 'o',
        'ß' => 's',
        _ => return None,
    };
    Some(folded)
}

pub fn flip_last_first(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((last, first)) = trimmed.split_once(',') {
        let last = last.trim();
        let first = first.trim();
        if !last.is_empty() && !first.is_empty() {
            return format!("{first} {last}");
        }
    }
    trimmed.to_string()
}
