use super::*;
use std::error::Error;

#[test]
fn projector_error_layout_stays_below_the_large_result_threshold() -> TestResult {
    check!(std::mem::size_of::<HtmlResultProjectionError>() < 128);
    Ok(())
}

#[test]
fn source_error_conversion_keeps_its_typed_capacity_and_display() -> TestResult {
    let source = RetainedResultOwnershipError::Capacity {
        needed: 65,
        available: 64,
    };
    let message = source.to_string();
    let converted = HtmlResultProjectionError::from(source);
    check!(
        matches!(&converted, HtmlResultProjectionError::Source(source)
        if matches!(source.as_ref(), RetainedResultOwnershipError::Capacity {
            needed: 65, available: 64
        }))
    );
    check!(eq; converted.to_string(), message);
    check!(converted.source().is_none());
    Ok(())
}

#[test]
fn source_error_conversion_preserves_the_transparent_store_source_chain() -> TestResult {
    let source = RetainedResultOwnershipError::Capture(census_store::StoreError::BudgetExceeded {
        what: "controlled typed source budget",
        needed: 2,
        available: 1,
    });
    let message = source.to_string();
    let source_message = source.source().ok_or("original store source")?.to_string();
    let converted = HtmlResultProjectionError::from(source);
    check!(eq; converted.to_string(), message);
    let chained = converted.source().ok_or("forwarded store source")?;
    check!(eq; chained.to_string(), source_message);
    check!(matches!(
        chained.downcast_ref::<census_store::StoreError>(),
        Some(census_store::StoreError::BudgetExceeded {
            what: "controlled typed source budget",
            needed: 2,
            available: 1
        })
    ));
    Ok(())
}
