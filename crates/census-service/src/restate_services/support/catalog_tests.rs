use super::JobError;
use census_domain::model::{CatalogError, Id};
use census_report::report::ReportError;

fn error(source: CatalogError) -> ReportError {
    ReportError::Catalog {
        event: Id::mint("event", &["catalog-error-retry-class"]),
        source,
    }
}

#[test]
fn malformed_catalog_boundaries_are_terminal_and_keep_the_event_context() {
    for source in [
        CatalogError::LabelCountExceeded,
        CatalogError::LabelBytesExceeded,
        CatalogError::AggregateLabelBytesExceeded,
        CatalogError::AggregateLabelBytesOverflow,
        CatalogError::InvalidLabel,
    ] {
        let refused = error(source);
        let expected = refused.to_string();
        assert!(
            matches!(JobError::from(refused), JobError::Terminal { message } if message == expected)
        );
    }
}

#[test]
fn catalog_allocation_exhaustion_remains_transient_with_its_original_context() {
    let exhausted = error(CatalogError::ResourceExhausted);
    let expected = exhausted.to_string();
    assert!(
        matches!(JobError::from(exhausted), JobError::Transient { message } if message == expected)
    );
}
