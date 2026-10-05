use super::patterns::{BAD_EMAIL_CHARS, INVISIBLE, MAX_ANCHORS, TITLE_MAX, TLD_OK, TRIM_CHARS};
use super::Rules;

impl Rules {
    pub fn sanitize_email(&self, raw: &str) -> Option<String> {
        let value = self.normalized_email(raw)?;
        let (local, domain) = value.rsplit_once('@')?;
        if !self.local.is_match(local) || self.local.find(local).map(|m| m.as_str()) != Some(local)
        {
            return None;
        }
        if !self.domain.is_match(domain)
            || self.domain.find(domain).map(|m| m.as_str()) != Some(domain)
        {
            return None;
        }
        if !tld_ok(domain) {
            return None;
        }
        Some(value.to_ascii_lowercase())
    }

    fn normalized_email(&self, raw: &str) -> Option<String> {
        let unescaped = decode_entities(raw);
        let cleaned: String = unescaped
            .chars()
            .filter(|ch| !INVISIBLE.contains(ch))
            .collect();
        let mut value = cleaned.trim().to_string();
        if value.to_ascii_lowercase().starts_with("mailto:") {
            value = value.get(7..).map_or(String::new(), str::to_string);
        }
        let value = value
            .trim_matches(|ch: char| TRIM_CHARS.contains(&ch))
            .to_string();
        if value.is_empty() || value.matches('@').count() != 1 || value.contains("..") {
            return None;
        }
        if value.chars().any(|ch| BAD_EMAIL_CHARS.contains(&ch)) {
            return None;
        }
        Some(value)
    }

    pub fn sanitize_emails<'a>(&self, values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let mut out = Vec::new();
        for value in values {
            let Some(email) = self.sanitize_email(value) else {
                continue;
            };
            if out.contains(&email) {
                continue;
            }
            out.push(email);
        }
        out
    }

    pub fn visible_text(&self, html: &str) -> String {
        let stripped = self.hidden_block.replace_all(html, " ");
        let stripped = self.comment.replace_all(&stripped, " ");
        let stripped = self.tag.replace_all(&stripped, " ");
        let decoded = decode_entities(&stripped);
        let collapsed = self.inline_gap.replace_all(&decoded, " ");
        let mut out = String::with_capacity(collapsed.len());
        let mut newlines = 0usize;
        for ch in collapsed.chars() {
            if ch == '\n' || ch == '\r' {
                newlines = newlines.saturating_add(1);
                continue;
            }
            if newlines > 0 {
                out.push('\n');
                newlines = 0;
            }
            out.push(ch);
        }
        out.trim().to_string()
    }

    pub fn flat_text(&self, value: &str) -> String {
        self.inline_gap.replace_all(value, " ").to_string()
    }

    pub fn collapse_spaces(&self, value: &str) -> String {
        self.any_gap.replace_all(value.trim(), " ").to_string()
    }

    pub fn title_of(&self, html: &str) -> String {
        let raw = self
            .title_tag
            .captures(html)
            .and_then(|captures| captures.get(1))
            .map_or("", |matched| matched.as_str());
        let flattened = self.collapse_spaces(&decode_entities(&self.tag.replace_all(raw, " ")));
        truncate(&flattened, TITLE_MAX).to_string()
    }

    pub fn anchors(&self, html: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for captures in self.anchor.captures_iter(html) {
            if out.len() >= MAX_ANCHORS {
                break;
            }
            let attributes = captures.get(1).map_or("", |matched| matched.as_str());
            let inner = captures.get(2).map_or("", |matched| matched.as_str());
            let href = attribute(self, attributes, "href");
            if href.is_empty() {
                continue;
            }
            let label = self.collapse_spaces(&decode_entities(&self.tag.replace_all(inner, " ")));
            out.push((href.clone(), label));
        }
        out
    }

    pub fn anchor_candidates(&self, html: &str) -> Vec<(String, String)> {
        self.anchors(html)
            .into_iter()
            .filter(|(href, label)| {
                !href.is_empty()
                    && !href.starts_with("mailto:")
                    && !href.starts_with("tel:")
                    && !href.starts_with("javascript:")
                    && self.interesting(href, label)
            })
            .collect()
    }
}

fn tld_ok(domain: &str) -> bool {
    let tld = domain
        .rsplit('.')
        .next()
        .map_or("", |part| part)
        .to_ascii_lowercase();
    let two_alpha = tld.len() == 2 && tld.chars().all(|ch| ch.is_ascii_alphabetic());
    TLD_OK.contains(&tld.as_str()) || two_alpha
}

fn or_empty(value: Option<&str>) -> &str {
    value.map_or("", std::convert::identity)
}

pub(super) fn attribute(rules: &Rules, tag: &str, name: &str) -> String {
    rules
        .attribute
        .captures_iter(tag)
        .find(|captures| {
            captures
                .get(1)
                .is_some_and(|matched| matched.as_str().eq_ignore_ascii_case(name))
        })
        .and_then(|captures| captures.get(2).or_else(|| captures.get(3)))
        .map_or(String::new(), |matched| matched.as_str().trim().to_string())
}

pub fn truncate(value: &str, max: usize) -> &str {
    if value.len() <= max {
        return value;
    }
    let mut end = max;
    while end > 0 && !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    or_empty(value.get(..end))
}

pub(super) fn decode_entities(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(or_empty(rest.get(..index)));
        let tail = or_empty(rest.get(index..));
        let Some(semicolon) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        if semicolon > 12 {
            out.push('&');
            rest = or_empty(tail.get(1..));
            continue;
        }
        let entity = or_empty(tail.get(1..semicolon));
        push_entity(&mut out, entity);
        rest = or_empty(tail.get(semicolon.saturating_add(1)..));
    }
    out.push_str(rest);
    out
}

fn push_entity(out: &mut String, entity: &str) {
    match entity {
        "amp" => out.push('&'),
        "lt" => out.push('<'),
        "gt" => out.push('>'),
        "quot" => out.push('"'),
        "apos" | "#39" | "#x27" | "#X27" => out.push('\''),
        "nbsp" => out.push(' '),
        "mdash" => out.push('—'),
        "ndash" => out.push('–'),
        "rsquo" => out.push('’'),
        "lsquo" => out.push('‘'),
        "rdquo" => out.push('”'),
        "ldquo" => out.push('“'),
        other => push_numeric(out, other),
    }
}

fn push_numeric(out: &mut String, other: &str) {
    let numeric = other
        .strip_prefix("#x")
        .or_else(|| other.strip_prefix("#X"))
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .or_else(|| {
            other
                .strip_prefix('#')
                .and_then(|decimal| decimal.parse::<u32>().ok())
        });
    match numeric.and_then(char::from_u32) {
        Some(ch) => out.push(ch),
        None => {
            out.push('&');
            out.push_str(other);
            out.push(';');
        }
    }
}
