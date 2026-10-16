pub(super) struct RowLocator<'a> {
    pub(super) resource_url: &'a str,
    pub(super) capture_sha256: &'a str,
    pub(super) ordinal: u64,
    pub(super) byte_offset: usize,
    pub(super) byte_length: usize,
}

pub(super) fn parse(key: &str) -> super::TestResult<RowLocator<'_>> {
    let (resource_url, fragment) = key
        .split_once('#')
        .ok_or("row key missing fragment delimiter")?;
    if resource_url.is_empty() {
        return Err("row key has empty resource URL".into());
    }
    if !resource_url.starts_with("http") {
        return Err(format!("row key resource is not a URL: {resource_url}").into());
    }
    let mut sha256 = None;
    let mut row = None;
    let mut offset = None;
    let mut length = None;
    for component in fragment.split('&') {
        let (field, value) = component
            .split_once('=')
            .ok_or_else(|| format!("row key fragment component has no value: {component}"))?;
        match field {
            "capture" if sha256.is_none() => sha256 = Some(value),
            "row" if row.is_none() => {
                row = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| format!("row key ordinal is not an integer: {value}"))?,
                );
            }
            "offset" if offset.is_none() => {
                offset = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("row key offset is not an integer: {value}"))?,
                );
            }
            "length" if length.is_none() => {
                length = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| format!("row key length is not an integer: {value}"))?,
                );
            }
            _ => {
                return Err(
                    format!("row key has unexpected or duplicate fragment field: {field}").into(),
                )
            }
        }
    }
    Ok(RowLocator {
        resource_url,
        capture_sha256: sha256.ok_or("row key missing capture SHA-256")?,
        ordinal: row.ok_or("row key missing row ordinal")?,
        byte_offset: offset.ok_or("row key missing byte offset")?,
        byte_length: length.ok_or("row key missing byte length")?,
    })
}

pub(super) fn span<'a>(locator: &RowLocator<'_>, body: &'a [u8]) -> super::TestResult<&'a [u8]> {
    let end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or("row key span overflows usize")?;
    let span = body.get(locator.byte_offset..end).ok_or_else(|| {
        format!(
            "row key span [{offset},{end}) exceeds body length {len}",
            offset = locator.byte_offset,
            len = body.len()
        )
    })?;
    if span.is_empty() {
        return Err("row key span is empty".into());
    }
    Ok(span)
}

pub(super) fn assert_span_contains_profile(
    locator: &RowLocator<'_>,
    body: &[u8],
    expected_profile_url: &str,
) -> super::TestResult {
    let span = span(locator, body)?;
    let text = std::str::from_utf8(span).map_err(|_| "row key span is not valid UTF-8")?;
    if !text.contains(expected_profile_url) {
        return Err(format!(
            "row key span for profile {profile} does not contain the profile URL; span text: {text}",
            profile = expected_profile_url
        )
        .into());
    }
    Ok(())
}
