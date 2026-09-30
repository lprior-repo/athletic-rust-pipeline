pub const INDEX_ROW: &str =
    r#"<div class="title"><a href="profile\.php\?instid=([0-9A-Za-z]+)">([^<]*)</a></div>"#;

pub const PROFILE_TITLE: &str = r"<title>([^|]+) \| NYSED Data Site</title>";

pub const INSTITUTION_ID: &str = r"<strong>INSTITUTION ID: </strong>([0-9]+)";

pub const PHONE: &str = r#"<strong>PHONE: </strong><a href="tel:\+1([0-9]{10})""#;

pub const WEBSITE: &str = r#"<strong>WEBSITE: </strong><a href="([^"]+)""#;

pub const MAPS_QUERY: &str = r#"maps/embed[^"]*?q=([^"&]+)"#;

pub const TOTAL_STUDENTS: &str = r#"id="total_students"[^>]*>(?:\s*<span[^>]*>)?\s*([0-9]+)"#;

pub fn percent_decode(raw: &str) -> String {
    let mut bytes: Vec<u8> = Vec::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut index = 0usize;
    while let Some(ch) = chars.get(index) {
        match ch {
            '+' => bytes.push(b' '),
            '%' => {
                let pair: String = [
                    chars.get(index.saturating_add(1)),
                    chars.get(index.saturating_add(2)),
                ]
                .into_iter()
                .flatten()
                .collect();
                match u8::from_str_radix(&pair, 16) {
                    Ok(byte) => bytes.push(byte),
                    Err(_) => bytes.extend(ch.to_string().bytes()),
                }
                index = index.saturating_add(2);
            }
            other => bytes.extend(other.to_string().bytes()),
        }
        index = index.saturating_add(1);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
