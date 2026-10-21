use super::{invalid, markup, shifted, year, RoncalliStatusFactV1};
use census_store::StoreResult;
use std::ops::Range;

mod grade;

pub(super) fn read(html: &str, selected: Range<usize>) -> StoreResult<RoncalliStatusFactV1> {
    let article = markup::single(markup::elements(html, b"article", None)?)?;
    let title = markup::single(markup::elements(article.html, b"h1", None)?)?;
    if markup::text(title.html)? != "Girls Track & Field" {
        return Err(invalid("Roncalli status sport heading differs"));
    }
    let (season_year, heading, section) = section(&article)?;
    let paragraphs = markup::elements(article.html, b"p", None)?;
    let mut found = None;
    for paragraph in &paragraphs {
        if paragraph.span.start < section.start || paragraph.span.end > section.end {
            continue;
        }
        let base = paragraph
            .span
            .start
            .checked_add(article.span.start)
            .ok_or(census_store::StoreError::CounterOverflow)?;
        for candidate in grade::read(paragraph)? {
            let span = shifted(candidate.span, base)?;
            if span == selected {
                if found.is_some() {
                    return Err(invalid("Roncalli selected status subject is ambiguous"));
                }
                let grade = candidate
                    .grade
                    .ok_or_else(|| invalid("Roncalli selected explicit status subject absent"))?;
                found = Some(RoncalliStatusFactV1 {
                    name: candidate.name,
                    grade,
                    season_year,
                    heading_span: shifted(heading.clone(), article.span.start)?,
                    statement_span: shifted(paragraph.span.clone(), article.span.start)?,
                    subject_span: span,
                });
            }
        }
    }
    let fact = found.ok_or_else(|| invalid("Roncalli selected explicit status subject absent"))?;
    conflicts(&section, &paragraphs, &fact)?;
    Ok(fact)
}

fn section(article: &markup::Element<'_>) -> StoreResult<(u16, Range<usize>, Range<usize>)> {
    let headings = markup::elements(article.html, b"h3", None)?;
    let mut selected = None;
    for heading in &headings {
        let text = markup::text(heading.html)?;
        let Some(edition) = text.strip_suffix(" Season Review") else {
            continue;
        };
        if selected.is_some() {
            return Err(invalid("Roncalli season review heading is ambiguous"));
        }
        selected = Some((year(edition)?, heading.span.clone()));
    }
    let (year, heading) =
        selected.ok_or_else(|| invalid("Roncalli season review heading absent"))?;
    let end = headings
        .iter()
        .filter(|item| item.span.start >= heading.end)
        .map(|item| item.span.start)
        .min()
        .map_or(article.html.len(), |end| end);
    Ok((year, heading.clone(), heading.end..end))
}

fn conflicts(
    section: &Range<usize>,
    paragraphs: &[markup::Element<'_>],
    selected: &RoncalliStatusFactV1,
) -> StoreResult<()> {
    for paragraph in paragraphs {
        if paragraph.span.start < section.start || paragraph.span.end > section.end {
            continue;
        }
        for candidate in grade::read(paragraph)? {
            if candidate.name == selected.name && candidate.grade != Some(selected.grade) {
                return Err(invalid("Roncalli explicit student grades conflict"));
            }
        }
    }
    Ok(())
}
