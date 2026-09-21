//! HTML and published-value primitives for the WIAA directory payloads.
//!
//! No I/O and no store access: every function takes captured text and returns text, an offset
//! or a decoded address, so the parsers above it stay fixture-testable.

// -------------------------------------------------------------------------------------------------
// String primitives (no regex: nothing here can panic, every offset is bounds-checked)
// -------------------------------------------------------------------------------------------------

/// Byte offset of `needle` at or after `from`.
pub(super) fn find_from(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .find(needle)
        .and_then(|offset| from.checked_add(offset))
}

/// Text of the first `<tag …>…</tag>` at or after `from`, plus the offset just past it.
pub(super) fn element_text(html: &str, tag: &str, from: usize) -> Option<(String, usize)> {
    let open_marker = format!("<{tag}");
    let close_marker = format!("</{tag}>");
    let open = find_from(html, &open_marker, from)?;
    let open_end = find_from(html, ">", open)?.checked_add(1)?;
    let close = find_from(html, &close_marker, open_end)?;
    let text = html
        .get(open_end..close)
        .map(strip_tags)
        .unwrap_or_default();
    let next = close.checked_add(close_marker.len())?;
    Some((text, next))
}

/// Bodies of every `<tag …>…</tag>` inside `html`, in document order. The cursor always advances to
/// just past a closing tag, so the loop terminates on any input.
pub(super) fn element_bodies<'a>(html: &'a str, tag: &str) -> Vec<&'a str> {
    let open_marker = format!("<{tag}");
    let close_marker = format!("</{tag}>");
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(open) = find_from(html, &open_marker, cursor) {
        let Some(open_end) = find_from(html, ">", open).and_then(|at| at.checked_add(1)) else {
            break;
        };
        let Some(close) = find_from(html, &close_marker, open_end) else {
            break;
        };
        if let Some(body) = html.get(open_end..close) {
            out.push(body);
        }
        match close.checked_add(close_marker.len()) {
            Some(next) if next > cursor => cursor = next,
            _ => break,
        }
    }
    out
}

/// First `attribute="value"` inside `html`.
pub(super) fn attribute_value(html: &str, attribute: &str) -> Option<String> {
    let marker = format!("{attribute}=\"");
    let start = find_from(html, &marker, 0)?.checked_add(marker.len())?;
    let rest = html.get(start..)?;
    let end = rest.find('"')?;
    rest.get(..end).map(|value| clean(&unescape(value)))
}

/// Text of every `<label class="<class>">…</label>` inside `html`, in document order.
pub(super) fn labelled_texts(html: &str, class: &str) -> Vec<String> {
    let marker = format!("<label class=\"{class}\">");
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(at) = find_from(html, &marker, cursor) {
        let Some(start) = at.checked_add(marker.len()) else {
            break;
        };
        let Some(end) = find_from(html, "</label>", start) else {
            break;
        };
        if let Some(text) = html.get(start..end) {
            out.push(strip_tags(text));
        }
        match end.checked_add("</label>".len()) {
            Some(next) if next > cursor => cursor = next,
            _ => break,
        }
    }
    out
}

/// The `<table id="<table_id>">…</table>` slice, starting at the id attribute.
pub(super) fn table_slice<'a>(html: &'a str, table_id: &str) -> Option<&'a str> {
    let marker = format!("id=\"{table_id}\"");
    let at = find_from(html, &marker, 0)?;
    let end = find_from(html, "</table>", at)?;
    html.get(at..end)
}

/// Cell `index` of a row, or `""` when the row is shorter than that.
pub(super) fn nth<'a>(cells: &[&'a str], index: usize) -> &'a str {
    cells.get(index).copied().unwrap_or("")
}

/// Element `index` of an owned list, or `""` when the list is shorter than that.
pub(super) fn nth_owned(values: &[String], index: usize) -> &str {
    values.get(index).map(String::as_str).unwrap_or("")
}

/// Remove markup, decode the entities this site emits, and collapse whitespace.
pub(super) fn strip_tags(fragment: &str) -> String {
    let mut out = String::with_capacity(fragment.len());
    let mut depth = 0usize;
    for ch in fragment.chars() {
        match ch {
            '<' => depth = depth.saturating_add(1),
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    clean(&unescape(&out))
}

/// Decode the HTML entities observed in WIAA payloads.
fn unescape(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&times;", "×")
}

fn collapse_whitespace(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_space = false;
    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            in_space = true;
        } else {
            if in_space && !out.is_empty() {
                out.push(' ');
            }
            in_space = false;
            out.push(ch);
        }
    }
    out
}

pub(super) fn clean(value: &str) -> String {
    collapse_whitespace(value)
}

/// `None` for an empty or whitespace value, so optional fields stay absent rather than empty.
pub(super) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// A published value that is not a placeholder. WIAA writes `N/A` where a field does not apply
/// (e.g. the conference of a charter school), and a placeholder must not become a canonical field.
pub(super) fn meaningful(value: &str) -> Option<String> {
    nonempty(value).filter(|value| {
        !matches!(
            value.to_ascii_lowercase().as_str(),
            "n/a" | "na" | "none" | "-" | "--" | "unknown" | "tbd"
        )
    })
}

// -------------------------------------------------------------------------------------------------
// Email decoding
// -------------------------------------------------------------------------------------------------

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte.checked_sub(b'a')?.checked_add(10),
        b'A'..=b'F' => byte.checked_sub(b'A')?.checked_add(10),
        _ => None,
    }
}

/// Decode Cloudflare's `data-cfemail` payload exactly as the page's own `email-decode.min.js` does:
/// the first byte is the XOR key, every following byte is one character of the address.
///
/// Returns `None` for malformed input and for a payload that does not decode to an address, so a
/// junk attribute can never become a `professional_email`.
pub fn decode_cfemail(encoded: &str) -> Option<String> {
    let hex = encoded.trim();
    let bytes = hex.as_bytes();
    if bytes.len() < 4 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut decoded_bytes = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(*pair.first()?)?;
        let low = hex_nibble(*pair.get(1)?)?;
        decoded_bytes.push(high.checked_shl(4)?.checked_add(low)?);
    }
    let key = *decoded_bytes.first()?;
    let decoded: String = decoded_bytes
        .iter()
        .skip(1)
        .map(|byte| char::from(*byte ^ key))
        .collect();
    valid_email(&decoded)
}

/// A published address, or nothing: rejects blanks, placeholders and anything without a domain dot.
pub(super) fn valid_email(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return None;
    }
    let mut parts = trimmed.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if parts.next().is_some() || local.is_empty() || !domain.contains('.') {
        return None;
    }
    if trimmed.contains(['[', ']']) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Address from a table cell: the Cloudflare payload when present, otherwise a plain `mailto:` href.
pub(super) fn cell_email(cell: &str) -> Option<String> {
    let marker = "data-cfemail=\"";
    if let Some(start) = find_from(cell, marker, 0).and_then(|at| at.checked_add(marker.len())) {
        if let Some(rest) = cell.get(start..) {
            if let Some(end) = rest.find('"') {
                if let Some(decoded) = rest.get(..end).and_then(decode_cfemail) {
                    return Some(decoded);
                }
            }
        }
    }
    let mailto = find_from(cell, "mailto:", 0).and_then(|at| at.checked_add("mailto:".len()))?;
    let rest = cell.get(mailto..)?;
    let end = rest.find(['"', '\'', '<', '?']).unwrap_or(rest.len());
    valid_email(rest.get(..end)?)
}
