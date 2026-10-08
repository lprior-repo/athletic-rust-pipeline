pub const INDEX_ROW: &str =
    r#"<div class="title"><a href="profile\.php\?instid=([^"]*)">([^<]*)</a></div>"#;

pub const PROFILE_TITLE: &str = r"<title>([^|]+) \| NYSED Data Site</title>";

pub const INSTITUTION_ID: &str = r"<strong>INSTITUTION ID: </strong>([^<\r\n]*)";

pub const PHONE: &str = r#"<strong>PHONE: </strong><a href="tel:\+1([0-9]{10})""#;

pub const WEBSITE: &str = r#"<strong>WEBSITE: </strong><a href="([^"]+)""#;

pub const MAPS_QUERY: &str = r#"maps/embed[^"]*?q=([^"&]+)"#;

pub const TOTAL_STUDENTS: &str = r#"id="total_students"[^>]*>(?:\s*<span[^>]*>)?\s*([0-9]+)"#;

pub fn percent_decode(
    raw: &str,
) -> Result<String, census_domain::school_directory::DirectoryError> {
    use census_domain::school_directory::DirectoryError;
    if raw.len() > 4096 {
        return Err(DirectoryError::Capacity {
            resource: "NYSED map query bytes",
            requested: raw.len(),
            limit: 4096,
        });
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve(raw.len())
        .map_err(|_| DirectoryError::Allocation {
            resource: "NYSED map query",
        })?;
    let mut input = raw.bytes();
    std::iter::from_fn(|| decoded_byte(&mut input)).for_each(|byte| bytes.push(byte));
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) => Err(DirectoryError::Representation {
            detail: crate::directory::issue_detail(format_args!("{}", error.utf8_error()))?,
        }),
    }
}

fn decoded_byte(input: &mut std::str::Bytes<'_>) -> Option<u8> {
    let byte = input.next()?;
    match byte {
        b'+' => Some(b' '),
        b'%' => {
            let mut preview = input.clone();
            let pair = preview
                .next()
                .and_then(hex)
                .zip(preview.next().and_then(hex));
            match pair {
                Some((high, low)) => {
                    *input = preview;
                    Some(high * 16 + low)
                }
                None => Some(byte),
            }
        }
        other => Some(other),
    }
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|value| value.checked_add(10)),
        b'A'..=b'F' => byte
            .checked_sub(b'A')
            .and_then(|value| value.checked_add(10)),
        _ => None,
    }
}
