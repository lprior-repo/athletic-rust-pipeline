use super::{bounded, invalid, Gender, Result, RetainedResultOwnershipError, Sport};
use crate::participation_evidence::markup;
use std::ops::Range;
use url::Url;

pub(super) struct Link {
    pub(super) url: Url,
    pub(super) text: String,
    pub(super) span: Range<usize>,
}

pub(super) fn list(raw: &str) -> Result<(Url, String, i16, Sport)> {
    let url = Url::parse(raw).map_err(|_| invalid("TFRRS list URL"))?;
    let host = url.host_str().ok_or_else(|| invalid("TFRRS host"))?;
    if url.scheme() != "https"
        || !(host == "tfrrs.org" || host.ends_with(".tfrrs.org"))
        || url.as_str() != raw
    {
        return Err(invalid("TFRRS list origin"));
    }
    secure(&url)?;
    let (list_id, year, sport) = list_path(&url)?;
    Ok((url, list_id, year, sport))
}

fn list_path(url: &Url) -> Result<(String, i16, Sport)> {
    let parts = segments(url)?;
    if parts.len() != 5 || parts.first().copied() != Some("lists") {
        return Err(invalid("TFRRS list native path"));
    }
    let list_id = id(parts
        .get(1)
        .copied()
        .ok_or_else(|| invalid("TFRRS list ID"))?)?;
    slug(
        parts
            .get(2)
            .copied()
            .ok_or_else(|| invalid("TFRRS list slug"))?,
    )?;
    let raw_year = parts
        .get(3)
        .copied()
        .ok_or_else(|| invalid("TFRRS list year"))?;
    let year: i16 = raw_year.parse().map_err(|_| invalid("TFRRS list year"))?;
    if !(1900..=2999).contains(&year) || year.to_string() != raw_year {
        return Err(invalid("TFRRS list year"));
    }
    let sport = match parts.get(4).copied() {
        Some("i") => Sport::IndoorTrack,
        Some("o") => Sport::OutdoorTrack,
        _ => return Err(invalid("unsupported TFRRS list season")),
    };
    Ok((list_id, year, sport))
}

pub(super) fn single(html: &str, offset: usize, base: &Url) -> Result<Link> {
    let element = markup::single(markup::elements(html, b"a", None)?)?;
    let href = element
        .attribute("href")
        .ok_or_else(|| invalid("TFRRS native link href"))?;
    bounded(href.len(), 2048)?;
    if !(href.starts_with('/') || href.starts_with("https://"))
        || href.starts_with("//")
        || href.contains('%')
        || href.split('/').any(|part| part == "." || part == "..")
        || href
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return Err(invalid("TFRRS native link syntax"));
    }
    let url = base
        .join(href)
        .map_err(|_| invalid("TFRRS native link URL"))?;
    secure(&url)?;
    if href.starts_with("https://") && url.as_str() != href {
        return Err(invalid("noncanonical TFRRS native link"));
    }
    if url.origin() != base.origin() {
        return Err(invalid("foreign TFRRS native link"));
    }
    let text = markup::text(element.html)?;
    bounded(text.len(), 256)?;
    if text.is_empty() {
        return Err(invalid("empty TFRRS native link label"));
    }
    let start = offset
        .checked_add(element.span.start)
        .ok_or(RetainedResultOwnershipError::Overflow)?;
    let end = offset
        .checked_add(element.span.end)
        .ok_or(RetainedResultOwnershipError::Overflow)?;
    Ok(Link {
        url,
        text,
        span: start..end,
    })
}

pub(super) fn athlete(url: &Url) -> Result<(String, String)> {
    let parts = segments(url)?;
    if parts.len() != 4 || parts.first().copied() != Some("athletes") {
        return Err(invalid("TFRRS athlete native path"));
    }
    let athlete = id(parts
        .get(1)
        .copied()
        .ok_or_else(|| invalid("TFRRS athlete ID"))?)?;
    let team = parts
        .get(2)
        .copied()
        .ok_or_else(|| invalid("TFRRS athlete team path"))?;
    slug(team)?;
    let person = parts
        .get(3)
        .copied()
        .ok_or_else(|| invalid("TFRRS athlete name path"))?;
    slug(person)?;
    if !person.starts_with('_') || person.len() < 2 {
        return Err(invalid("TFRRS individual athlete path"));
    }
    Ok((athlete, team.into()))
}

pub(super) fn team(url: &Url, gender: Gender) -> Result<String> {
    let parts = segments(url)?;
    if parts.len() != 3
        || parts.first().copied() != Some("teams")
        || parts.get(1).copied() != Some("tf")
    {
        return Err(invalid("TFRRS team native path"));
    }
    let raw = parts
        .get(2)
        .copied()
        .ok_or_else(|| invalid("TFRRS team native slug"))?;
    let suffix = match gender {
        Gender::Boys => "_m.html",
        Gender::Girls => "_f.html",
        _ => return Err(invalid("TFRRS team gender")),
    };
    let name = raw
        .strip_suffix(suffix)
        .ok_or_else(|| invalid("TFRRS team gender mismatch"))?;
    slug(name)?;
    Ok(name.into())
}

pub(super) fn meet(url: &Url) -> Result<(String, String)> {
    let parts = segments(url)?;
    if parts.len() != 3 || parts.first().copied() != Some("results") {
        return Err(invalid("TFRRS meet native path"));
    }
    let meet = id(parts
        .get(1)
        .copied()
        .ok_or_else(|| invalid("TFRRS meet ID"))?)?;
    let name = parts
        .get(2)
        .copied()
        .ok_or_else(|| invalid("TFRRS meet slug"))?;
    slug(name)?;
    Ok((meet, name.into()))
}

pub(super) fn performance(url: &Url, meet: &str, meet_slug: &str, event: &str) -> Result<String> {
    let parts = segments(url)?;
    if parts.len() != 5
        || parts.first().copied() != Some("results")
        || parts.get(1).copied() != Some(meet)
        || parts.get(3).copied() != Some(meet_slug)
    {
        return Err(invalid("TFRRS performance does not belong to row meet"));
    }
    let performance = id(parts
        .get(2)
        .copied()
        .ok_or_else(|| invalid("TFRRS performance ID"))?)?;
    let event_slug = parts
        .get(4)
        .copied()
        .ok_or_else(|| invalid("TFRRS performance event"))?;
    slug(event_slug)?;
    let expected = event.split_ascii_whitespace().collect::<Vec<_>>().join("-");
    if event_slug != expected {
        return Err(invalid("TFRRS performance event differs from section"));
    }
    Ok(performance)
}

fn secure(url: &Url) -> Result<()> {
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path().contains('%')
        || url.path().contains('\\')
    {
        return Err(invalid("ambiguous TFRRS native URL"));
    }
    Ok(())
}

fn segments(url: &Url) -> Result<Vec<&str>> {
    bounded(url.path().len(), 2048)?;
    let path = url
        .path()
        .strip_prefix('/')
        .ok_or_else(|| invalid("TFRRS absolute path"))?;
    let mut parts = Vec::new();
    parts
        .try_reserve_exact(5)
        .map_err(|_| RetainedResultOwnershipError::Allocation)?;
    for part in path.split('/') {
        bounded(
            parts
                .len()
                .checked_add(1)
                .ok_or(RetainedResultOwnershipError::Overflow)?,
            5,
        )?;
        if part.is_empty() {
            return Err(invalid("empty TFRRS path component"));
        }
        parts.push(part);
    }
    Ok(parts)
}

fn id(raw: &str) -> Result<String> {
    bounded(raw.len(), 20)?;
    let value = raw
        .parse::<u64>()
        .map_err(|_| invalid("TFRRS native numeric ID"))?;
    if value == 0 || value.to_string() != raw {
        return Err(invalid("noncanonical TFRRS native ID"));
    }
    Ok(raw.into())
}

fn slug(raw: &str) -> Result<()> {
    bounded(raw.len(), 256)?;
    if raw.is_empty()
        || !raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(invalid("unsupported TFRRS native slug"));
    }
    Ok(())
}
