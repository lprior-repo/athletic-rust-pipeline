use super::*;

// -------------------------------------------------------------------------------------------------
// Normalization
// -------------------------------------------------------------------------------------------------

/// Normalize a person/school/meet name for identity comparisons: lowercase, strip diacritics and
/// punctuation, collapse whitespace, drop school-type suffixes that vary between sources
/// ("high school", "hs", "school", "academy" is *kept* because it is distinguishing).
///
/// Suffixes are dropped until none applies, so the result is a fixpoint: normalizing an
/// already-normalized name is the identity. `SchoolId::mint` keys the identity on this string, so
/// two adapters handing the same school over in different spellings have to agree even when one of
/// them re-normalizes a name the other passed through raw.
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

/// Strip one trailing school-type suffix from `parts`, reporting whether anything was removed.
///
/// One call removes at most one suffix; [`normalize_name`] calls this until it returns `false`, so
/// a name that ends in several of them ("X High School School") reaches the same key as the name
/// without them instead of stopping one suffix short.
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
            // Guarded above (`parts.len() > tokens.len()`), so saturating is exact here.
            parts.truncate(parts.len().saturating_sub(tokens.len()));
            return true;
        }
    }
    false
}

fn strip_diacritic(ch: char) -> Option<char> {
    // A tiny, deterministic folding table is enough for Midwest school/person names; anything else
    // falls back to the unaccented ASCII range when possible.
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

/// Sort a `"Last, First"` roster name into `"First Last"`; leave other shapes alone.
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

/// Convenience map for counters used by adapter reports.
pub type Counters = BTreeMap<String, u64>;
