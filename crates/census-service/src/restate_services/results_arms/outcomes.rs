use census_crawl::CollectionDisposition;

use super::ResultsSourceRows;

pub(super) fn source_complete(source: &ResultsSourceRows) -> bool {
    source.disposition.is_complete()
        && source.rows.is_some()
        && source.errors == 0
        && source.withheld == Some(0)
        && source.unfinished.is_empty()
        && source
            .unresolved
            .is_some_and(|value| value.rows == 0 && value.labels == 0)
        && source.resolution.is_some_and(|value| value.unresolved == 0)
}

pub(super) fn delegated_source(slug: &str) -> ResultsSourceRows {
    ResultsSourceRows {
        slug: slug.to_string(),
        meets: 0,
        rows: None,
        disposition: CollectionDisposition::Unknown,
        errors: 0,
        withheld: None,
        notes: Vec::new(),
        unfinished: vec![format!("{slug}/acquisition-stage-outcome")],
        unresolved: None,
        resolution: None,
    }
}
