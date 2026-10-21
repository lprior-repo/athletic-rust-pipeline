use super::{bound, invalid, markup, shifted, year, RoncalliEventFactV1};
use census_domain::model::ProfileDate;
use census_store::StoreResult;

mod mark;

pub(super) fn read(html: &str, recap_id: &str, name: &str) -> StoreResult<RoncalliEventFactV1> {
    identifier(recap_id)?;
    let heading = markup::single(markup::elements(html, b"h1", Some("section_title"))?)?;
    let text = markup::text(heading.html)?;
    let edition = text
        .strip_suffix(" Girls Track & Field Schedule")
        .ok_or_else(|| invalid("Roncalli calendar sport heading differs"))?;
    let season_year = year(edition)?;
    unique_recap(html, recap_id)?;
    let event = selected_event(html, recap_id)?;
    bound(event.html.len(), 64 * 1024, "Roncalli selected event bytes")?;
    if event.html.to_ascii_lowercase().contains("cancel") {
        return Err(invalid("Roncalli selected event is canceled"));
    }
    let recap = markup::single(markup::elements(event.html, b"div", Some("modal"))?)?;
    if recap.attribute("id") != Some(recap_id) {
        return Err(invalid("Roncalli selected event recap differs"));
    }
    recap_link(&event, recap_id)?;
    let paragraph = markup::single(markup::elements(recap.html, b"p", None)?)?;
    let recap_text = markup::text(paragraph.html)?;
    let performance = mark::read(&recap_text, name)?;
    Ok(RoncalliEventFactV1 {
        season_year,
        day: day(&event, season_year)?,
        competition: field(&event, b"p", "title")?,
        venue: field(&event, b"p", "secondary_title")?,
        event_label: performance.event_label,
        original_mark: performance.original_mark,
        original_centiseconds: performance.centiseconds,
        place: performance.place,
        recap_id: recap_id.into(),
        recap_text,
        heading_span: heading.span,
        event_span: event.span.clone(),
        recap_span: shifted(recap.span, event.span.start)?,
    })
}

fn identifier(value: &str) -> StoreResult<()> {
    let digits = value
        .strip_prefix("event-")
        .and_then(|value| value.strip_suffix("-recap"))
        .ok_or_else(|| invalid("Roncalli calendar recap identity invalid"))?;
    if digits.is_empty()
        || digits.len() > 16
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid("Roncalli calendar recap identity invalid"));
    }
    Ok(())
}

fn unique_recap(html: &str, id: &str) -> StoreResult<()> {
    let matches = markup::elements(html, b"div", Some("modal"))?
        .into_iter()
        .filter(|element| element.attribute("id") == Some(id))
        .count();
    if matches != 1 {
        return Err(invalid(
            "Roncalli calendar recap identity missing or ambiguous",
        ));
    }
    Ok(())
}

fn selected_event<'a>(html: &'a str, id: &str) -> StoreResult<markup::Element<'a>> {
    let mut selected = None;
    for event in markup::elements(html, b"div", Some("event"))? {
        let matches = markup::elements(event.html, b"div", Some("modal"))?
            .into_iter()
            .any(|element| element.attribute("id") == Some(id));
        if matches {
            if selected.is_some() {
                return Err(invalid("Roncalli calendar parent event ambiguous"));
            }
            selected = Some(event);
        }
    }
    selected.ok_or_else(|| invalid("Roncalli calendar parent event absent"))
}

fn recap_link(event: &markup::Element<'_>, id: &str) -> StoreResult<()> {
    let expected = format!("#{id}");
    let links = markup::elements(event.html, b"a", None)?
        .into_iter()
        .filter(|link| link.attribute("href") == Some(expected.as_str()))
        .count();
    if links != 1 {
        return Err(invalid("Roncalli event own recap link differs"));
    }
    Ok(())
}

fn field(event: &markup::Element<'_>, tag: &[u8], class: &str) -> StoreResult<String> {
    let element = markup::single(markup::elements(event.html, tag, Some(class))?)?;
    let text = markup::text(element.html)?;
    bound(text.len(), 256, "Roncalli event field bytes")?;
    if text.is_empty() {
        return Err(invalid("Roncalli event field absent"));
    }
    Ok(text)
}

fn day(event: &markup::Element<'_>, year: u16) -> StoreResult<ProfileDate> {
    let date = markup::single(markup::elements(event.html, b"div", Some("date"))?)?;
    let time = markup::single(markup::elements(date.html, b"p", Some("time"))?)?;
    let prefix = date
        .html
        .get(..time.span.start)
        .ok_or_else(|| invalid("Roncalli event date span invalid"))?;
    let suffix = date
        .html
        .get(time.span.end..)
        .ok_or_else(|| invalid("Roncalli event date span invalid"))?;
    if !markup::text(suffix)?.is_empty() || markup::text(time.html)?.is_empty() {
        return Err(invalid("Roncalli event date or time ambiguous"));
    }
    let text = markup::text(prefix)?;
    let date = chrono::NaiveDate::parse_from_str(&format!("{year} {text}"), "%Y %a %b %e")
        .map_err(|_| invalid("Roncalli event day or weekday invalid"))?;
    ProfileDate::parse(&date.format("%Y-%m-%d").to_string())
        .map_err(|_| invalid("Roncalli event day unsupported"))
}
