pub fn normalize_name(raw: &str) -> String {
    let raw = raw.trim();
    let mut out = String::with_capacity(raw.len());
    let mut last_was_space = true;

    for ch in raw.chars() {
        for lc_ch in ch.to_lowercase() {
            let out_ch = if lc_ch.is_ascii_alphanumeric() {
                Some(lc_ch)
            } else if lc_ch.is_whitespace()
                || lc_ch == '-'
                || lc_ch == '\''
                || lc_ch == '.'
                || lc_ch == ','
                || lc_ch == '/'
            {
                Some(' ')
            } else if !lc_ch.is_ascii() {
                strip_diacritic(lc_ch)
            } else {
                None
            };

            if let Some(c) = out_ch {
                if c == ' ' {
                    if !last_was_space {
                        out.push(' ');
                        last_was_space = true;
                    }
                } else {
                    out.push(c);
                    last_was_space = false;
                }
            }
        }
    }

    if let Some(prefix) = out.strip_suffix(' ') {
        let length = prefix.len();
        out.truncate(length);
    }
    while strip_type_suffix(&mut out) {}

    out
}

fn strip_type_suffix(out: &mut String) -> bool {
    for suffix in [
        " high school",
        " hs",
        " highschool",
        " school",
        " sr high",
        " senior high",
    ] {
        if let Some(prefix) = out.strip_suffix(suffix) {
            if prefix.is_empty() {
                continue;
            }
            let length = prefix.len();
            out.truncate(length);
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
