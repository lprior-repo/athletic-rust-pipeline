use super::patterns::{MAX_TABLES, MAX_TABLE_ROWS};
use super::text::decode_entities;
use super::Rules;
use crate::{CrawlError, CrawlResult};

pub(super) struct CandidateTable<'a> {
    pub(super) rows: Vec<&'a str>,
}

pub(super) fn discover<'a>(rules: &Rules, html: &'a str) -> CrawlResult<Vec<CandidateTable<'a>>> {
    let mut tables = Vec::new();
    for table in rules.table.captures_iter(html) {
        let Some(body) = table.get(1) else {
            continue;
        };
        let mut rows = rules
            .row
            .captures_iter(body.as_str())
            .filter_map(|captures| captures.get(1))
            .map(|matched| matched.as_str())
            .peekable();
        let Some(header) = rows.next() else {
            continue;
        };
        if rows.peek().is_none() || !rules.table_header.is_match(&header_of(rules, header)) {
            continue;
        }
        if tables.len() >= MAX_TABLES {
            return Err(CrawlError::Resource {
                resource: "school site tables",
                requested: tables.len().saturating_add(1),
                limit: MAX_TABLES,
            });
        }
        tables.push(CandidateTable {
            rows: rows.take(MAX_TABLE_ROWS).collect(),
        });
    }
    Ok(tables)
}

fn header_of(rules: &Rules, row: &str) -> String {
    rules
        .collapse_spaces(&decode_entities(&rules.tag.replace_all(row, " ")))
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests;
