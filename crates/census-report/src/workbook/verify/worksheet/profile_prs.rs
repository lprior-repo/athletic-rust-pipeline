use super::invalid;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::verify::canonical::Value;
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeSet;

pub(super) fn value(dataset: &ExportDataset, athlete: &CanonicalAthlete) -> ReportResult<Value> {
    let mut expected = BTreeSet::new();
    let mut length = 0_usize;
    for table in dataset.reported_profile_prs_for(athlete) {
        for claim in &table.rows {
            let text = claim_literal(table, claim);
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
                return Err(invalid(
                    "frozen source PR claims exceed the Excel cell limit",
                ));
            }
            length = needed;
            expected.insert(text);
        }
    }
    let mut literal = String::new();
    for text in expected {
        if !literal.is_empty() {
            literal.push('\n');
        }
        literal.push_str(&text);
    }
    if literal.is_empty() {
        Ok(Value::Empty)
    } else {
        Ok(Value::text(literal))
    }
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
