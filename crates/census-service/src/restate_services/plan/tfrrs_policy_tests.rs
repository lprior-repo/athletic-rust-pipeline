use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn typed_source_holds_preserve_terms_and_access_blocked_plan_dispositions() -> TestResult {
    for (source, expected) in [
        ("tfrrs", RefusalKind::Terms),
        ("bound", RefusalKind::AccessBlocked),
    ] {
        let descriptor = census_crawl::registry::descriptor(source).ok_or("registered source")?;
        let reason = census_crawl::registry::acquisition_refusal_for_source(source)
            .ok_or("source hold reason")?;
        for lane in [BrowserLaneState::Absent, BrowserLaneState::Configured] {
            let UnitDisposition::Refused(refusal) = classify(descriptor, lane) else {
                return Err(format!("{source} became sweepable").into());
            };
            check!(eq; refusal.slug, source);
            check!(eq; refusal.kind, expected);
            check!(eq; refusal.reason, reason);
        }
    }
    check!(eq; refusal_kind("tfrrs"), RefusalKind::Terms);
    check!(eq; refusal_kind("dat"), RefusalKind::Terms);
    check!(eq;
        census_crawl::registry::acquisition_policy_for_source("dat"),
        census_crawl::registry::acquisition_policy_for_source("tfrrs")
    );
    check!(eq;
        unwired_reason("dat"),
        census_crawl::registry::acquisition_refusal_for_source("dat")
    );
    check!(eq;
        unwired_reason("tfrrs"),
        census_crawl::registry::acquisition_refusal_for_source("tfrrs")
    );
    Ok(())
}
