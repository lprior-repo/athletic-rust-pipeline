use super::super::map::SearchResult;
use super::super::parse::resolve_school_name;
use super::super::{Options, ASSOCIATION, HOST, SEARCH_PATH};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalSchool, SourceNamespace};
use std::collections::HashSet;

pub(super) async fn resolve_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<SearchResult>> {
    let mut to_process = if !options.school_names.is_empty() {
        search_schools(ctx, options, report).await?
    } else {
        let existing: Vec<census_domain::model::CanonicalSchool> =
            census_store::read::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
        existing
            .into_iter()
            .filter(|s| matches!(&s.association, Some(a) if a.as_str() == ASSOCIATION))
            .filter_map(|school| existing_school(school, report))
            .collect()
    };

    let mut seen_ids: HashSet<String> = HashSet::new();
    to_process.retain(|sr| seen_ids.insert(sr.ohsaa_id.clone()));
    if let Some(limit) = options.limit {
        to_process.truncate(limit);
    }

    Ok(to_process)
}

fn existing_school(school: CanonicalSchool, report: &mut AdapterReport) -> Option<SearchResult> {
    let namespace = SourceNamespace::association_school(ASSOCIATION);
    let owner = school.source_identities.iter().find(|identity| {
        identity.namespace == namespace
            && !identity.id.is_empty()
            && identity.id.bytes().all(|byte| byte.is_ascii_digit())
    });
    let Some(owner) = owner else {
        report.errors = report.errors.saturating_add(1);
        report.note(format!(
            "school {} has no usable OHSAA school owner; unfinished acquisition obligation",
            school.name
        ));
        return None;
    };
    Some(SearchResult {
        name: school.name,
        city: school.city.map_or(Default::default(), core::convert::identity),
        ohsaa_id: owner.id.clone(),
    })
}

async fn search_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<SearchResult>> {
    let mut results = Vec::new();
    for name in &options.school_names {
        let url = format!("{HOST}{SEARCH_PATH}?Name={}", url_encode(name));
        let mut notes: Vec<String> = Vec::new();
        match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
            Ok(outcome) if outcome.status == 200 => {
                if let Some(sr) = resolve_school_name(&outcome.text(), name, &mut notes) {
                    results.push(sr);
                } else {
                    report.note(format!("no school found for \"{}\"", name));
                }
                for n in notes {
                    report.note(n);
                }
            }
            Ok(outcome) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!(
                    "search \"{}\" returned HTTP {}",
                    name, outcome.status
                ));
            }
            Err(e) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!("search \"{}\": {}", name, e));
            }
        }
    }
    Ok(results)
}
fn url_encode(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(ch),
            ' ' => result.push_str("%20"),
            _ => {
                for byte in ch.to_string().bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}
