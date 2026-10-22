use super::{bound, invalid, markup, SourceProfilePrV1};
use census_domain::roster_affiliation_authority::SourceRowLocator;
use census_store::{StoreError, StoreResult};

pub(super) fn read(table: &markup::Element<'_>) -> StoreResult<Vec<SourceProfilePrV1>> {
    let mut output = Vec::new();
    for row in markup::elements(table.html, b"tr", None)? {
        let Some(mode) = row.attribute("data-pr-mode") else {
            if row.attribute("data-pr-level").is_some()
                || !markup::elements(row.html, b"td", None)?.is_empty()
            {
                return Err(invalid("profile PR advertised row mode absent"));
            }
            continue;
        };
        let mode = label(mode)?;
        let level = label(
            row.attribute("data-pr-level")
                .ok_or_else(|| invalid("profile PR level absent"))?,
        )?;
        let ordinal = output
            .len()
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        bound(ordinal, 256, "profile PR rows")?;
        output
            .try_reserve(1)
            .map_err(|_| invalid("profile PR allocation refused"))?;
        output.push(SourceProfilePrV1 {
            event: cell(&row, "pr-td-event")?,
            mark: cell(&row, "pr-td-mark")?,
            date: cell(&row, "pr-td-date")?,
            mode,
            level,
            locator: locator(table, &row, ordinal)?,
        });
    }
    Ok(output)
}

fn label(raw: &str) -> StoreResult<String> {
    bound(raw.len(), 32, "profile PR source label bytes")?;
    if raw.is_empty()
        || !raw
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
    {
        return Err(invalid("profile PR source label invalid"));
    }
    Ok(raw.into())
}

fn cell(row: &markup::Element<'_>, class: &str) -> StoreResult<String> {
    let cell = markup::single(markup::elements(row.html, b"td", Some(class))?)?;
    let text = markup::text(cell.html)?;
    bound(text.len(), 128, "profile PR cell bytes")?;
    if text.is_empty() {
        return Err(invalid("profile PR advertised cell empty"));
    }
    Ok(text)
}

fn locator(
    table: &markup::Element<'_>,
    row: &markup::Element<'_>,
    ordinal: usize,
) -> StoreResult<SourceRowLocator> {
    let offset = table
        .span
        .start
        .checked_add(row.span.start)
        .ok_or(StoreError::CounterOverflow)?;
    let length = row
        .span
        .end
        .checked_sub(row.span.start)
        .ok_or(StoreError::CounterOverflow)?;
    Ok(SourceRowLocator::new(
        u32::try_from(ordinal).map_err(|_| StoreError::CounterOverflow)?,
        offset,
        length,
    ))
}
