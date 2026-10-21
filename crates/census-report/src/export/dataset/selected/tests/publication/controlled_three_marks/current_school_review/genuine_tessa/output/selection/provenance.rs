use super::*;
use census_crawl::participation_evidence::result_ownership::tfrrs::{
    CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1,
};
use census_domain::model::EvidenceMethod;

pub(super) fn verify(performance: &CanonicalPerformance) -> TestResult {
    let capture = captures::tfrrs()?;
    let checked = read_retained_tfrrs_list_rows(&capture, &[source::locator()])?;
    let row = checked
        .row(0)
        .ok_or("retained genuine Tessa conversion row")?;
    check!(eq; row.performance_id(),"5841286");
    check!(eq; row.meet_id(),"94245");
    check!(eq; row.athlete_id(),"8779607");
    check!(eq; row.original_row_digest(),"d33bf3fb6858f07f8e41576232afd0bf89c848e420fbe67bef779f0be6f1df8d");
    check!(eq; row.original_mark(),Some("11:29.42"));
    check!(eq; row.published_mark(),"11:23.56");
    let native = performance
        .source_athlete
        .as_ref()
        .ok_or("native TFRRS athlete provenance")?;
    check!(eq; native.namespace,SourceNamespace::TfrrsAthlete);
    check!(eq; native.id,row.athlete_id());
    let url = format!("https://indiana.tfrrs.org{}", row.athlete_path());
    check!(eq; native.url.as_deref(),Some(url.as_str()));
    let adjustment = performance
        .adjustment
        .as_ref()
        .ok_or("retained genuine conversion")?;
    check!(eq; adjustment.source,None);
    check!(eq; adjustment.note.as_deref(),row.conversion_note());
    evidence(performance, &checked, &row)
}

fn evidence(
    performance: &CanonicalPerformance,
    checked: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
) -> TestResult {
    let capture = checked.capture();
    let expected = serde_json::json!({
        "interpretation":"tfrrs_html_physical_projection_v1",
        "admission":"unadmitted_projection",
        "content_key":performance.source_key,
        "list_context_reference":checked.context_reference(),
        "context_reference":row.context_reference(),
        "original_row_reference":row.reference(),
        "original_row_digest":row.original_row_digest(),
        "published_mark":row.published_mark(),
        "original_mark":row.original_mark(),
        "conversion_note":row.conversion_note(),
        "published_mark_span":row.published_mark_span(),
        "conversion_note_span":row.conversion_note_span()
    });
    let expected = expected.as_object().ok_or("conversion provenance fields")?;
    check!(!performance.evidence.is_empty());
    for evidence in &performance.evidence {
        check!(eq; evidence.source.id,"tfrrs_html_physical_projection_v1");
        check!(eq; evidence.source.url.as_deref(),Some(capture.requested_url()));
        check!(eq; evidence.method,EvidenceMethod::Parsed);
        check!(eq; evidence.observed_on,capture.acquired_at().to_rfc3339());
        let note: serde_json::Value = serde_json::from_str(
            evidence
                .note
                .as_deref()
                .ok_or("retained physical row evidence")?,
        )?;
        for (key, value) in expected {
            check!(eq; note.get(key),Some(value),"retained conversion provenance {key}");
        }
        let target = note
            .get("proposed_target")
            .ok_or("exact physical target evidence")?;
        check!(eq; target.get("locator"),Some(&serde_json::to_value(source::locator())?));
    }
    Ok(())
}
