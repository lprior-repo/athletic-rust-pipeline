use super::invalid;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::verify::canonical::Value;
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeSet;

pub(super) fn value(dataset: &ExportDataset, athlete: &CanonicalAthlete) -> ReportResult<Value> {
    let lines = dataset.reported_profile_prs_for(athlete).flat_map(|table| {
        table
            .rows
            .iter()
            .map(move |claim| claim_literal(table, claim))
    });
    if let Some(expected) = expected_detailed(lines)? {
        return Ok(expected);
    }
    expected_grouped(dataset, athlete)
}

fn expected_detailed(lines: impl Iterator<Item = String>) -> ReportResult<Option<Value>> {
    let mut expected = BTreeSet::new();
    let mut length = 0_usize;
    for text in lines {
        if expected.contains(&text) {
            continue;
        }
        let separator = usize::from(!expected.is_empty());
        let needed = text
            .encode_utf16()
            .count()
            .checked_add(separator)
            .and_then(|units| length.checked_add(units))
            .ok_or_else(|| invalid("frozen source PR cell length overflow"))?;
        if needed > 32_767 {
            return Ok(None);
        }
        length = needed;
        expected.insert(text);
    }
    let mut literal = String::new();
    for (index, text) in expected.into_iter().enumerate() {
        let bytes = text
            .len()
            .checked_add(usize::from(index > 0))
            .ok_or_else(|| invalid("frozen source PR cell byte length overflow"))?;
        literal
            .try_reserve(bytes)
            .map_err(|_| invalid("frozen source PR cell allocation failed"))?;
        if index > 0 {
            literal.push('\n');
        }
        literal.push_str(&text);
    }
    Ok(Some(if literal.is_empty() {
        Value::Empty
    } else {
        Value::Text(literal)
    }))
}

fn expected_grouped(dataset: &ExportDataset, athlete: &CanonicalAthlete) -> ReportResult<Value> {
    let mut literal = String::new();
    let mut length = 0_usize;
    for table in dataset.reported_profile_prs_for(athlete) {
        let header = format!(
            "Source-declared PR capture (unverified); Profile: {}; Capture: {}; Root: {}; Body interpretation: {}",
            table.profile_url, table.acquired_at.to_rfc3339(), table.root.key, table.interpretation,
        );
        append_expected(&mut literal, &mut length, &header)?;
        for claim in &table.rows {
            let location = claim.locator();
            let text = format!(
                "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Locator: {}:{}:{}",
                claim.event(), claim.published_mark(), claim.source_mode(), claim.source_level(),
                claim.published_date(), location.ordinal(), location.byte_offset(), location.byte_length(),
            );
            append_expected(&mut literal, &mut length, &text)?;
        }
    }
    Ok(Value::Text(literal))
}

fn append_expected(text: &mut String, units: &mut usize, line: &str) -> ReportResult<()> {
    let separator = usize::from(!text.is_empty());
    let needed = line
        .encode_utf16()
        .count()
        .checked_add(separator)
        .and_then(|length| units.checked_add(length))
        .ok_or_else(|| invalid("frozen source PR cell length overflow"))?;
    if needed > 32_767 {
        return Err(invalid(
            "complete grouped source-declared PR claims exceed Excel cell budget; no claims were truncated",
        ));
    }
    let additional = separator
        .checked_add(line.len())
        .ok_or_else(|| invalid("frozen source PR cell byte length overflow"))?;
    text.try_reserve(additional)
        .map_err(|_| invalid("frozen source PR cell allocation failed"))?;
    if separator != 0 {
        text.push('\n');
    }
    text.push_str(line);
    *units = needed;
    Ok(())
}

fn claim_literal(
    table: &crate::export::ReportedProfilePrTable,
    claim: &SourceProfilePrV1,
) -> String {
    let location = claim.locator();
    let mut text = format!(
        "{}: {}; Source-declared PR",
        claim.event(),
        claim.published_mark()
    );
    text.push_str(&format!(
        "; Mode: {}; Level: {}",
        claim.source_mode(),
        claim.source_level()
    ));
    text.push_str(&format!(
        "; Published date: {}; Profile: {}",
        claim.published_date(),
        table.profile_url
    ));
    text.push_str(&format!(
        "; Capture: {}; Root: {}",
        table.acquired_at.to_rfc3339(),
        table.root.key
    ));
    text.push_str(&format!(
        "; Body interpretation: {}; Locator: {}:{}:{}",
        table.interpretation,
        location.ordinal(),
        location.byte_offset(),
        location.byte_length()
    ));
    text
}

#[cfg(test)]
mod tests;
