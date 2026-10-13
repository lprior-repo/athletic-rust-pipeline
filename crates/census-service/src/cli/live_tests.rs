use super::*;
use census_service::restate_services::reject_offline_generation;
use census_service::restate_services::DEFAULT_GENERATION;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn the_default_generation_matches_the_legacy_constant() -> TestResult {
    let default_generation = ExportGeneration::default_generation();
    if default_generation.as_str() != DEFAULT_GENERATION {
        return Err(format!("default generation: {}", default_generation.as_str()).into());
    }
    let parsed = ExportGeneration::parse(DEFAULT_GENERATION)?;
    if default_generation != parsed {
        return Err(format!("default generation text: {}", parsed.as_str()).into());
    }
    Ok(())
}

#[test]
fn identical_semantic_requests_with_one_generation_share_retry_keys() -> TestResult {
    let generation = ExportGeneration::default_generation();
    let core = report_export_key("core", &[], &generation)?;
    if core != report_export_key("core", &[], &generation)? {
        return Err(format!("report key: {}", core.as_str()).into());
    }
    if core.as_str() != "report:core:all:1" {
        return Err(format!("report key: {}", core.as_str()).into());
    }
    let bests = bests_export_key("all", "2027", "50", &[], &generation)?;
    if bests != bests_export_key("all", "2027", "50", &[], &generation)? {
        return Err(format!("bests key: {}", bests.as_str()).into());
    }
    if bests.as_str() != "bests:all:2027:50:all:1" {
        return Err(format!("bests key: {}", bests.as_str()).into());
    }
    let consolidate = consolidate_export_key(&generation);
    if consolidate != consolidate_export_key(&generation) {
        return Err(format!("consolidate key: {}", consolidate.as_str()).into());
    }
    if consolidate.as_str() != "consolidate:1" {
        return Err(format!("consolidate key: {}", consolidate.as_str()).into());
    }
    Ok(())
}

#[test]
fn a_fresh_generation_produces_fresh_export_keys() -> TestResult {
    let previous = ExportGeneration::default_generation();
    let next = ExportGeneration::parse("2")?;
    let previous_report = report_export_key("core", &[], &previous)?;
    if previous_report == report_export_key("core", &[], &next)? {
        return Err(format!("report key: {}", previous_report.as_str()).into());
    }
    let next_report = report_export_key("core", &[], &next)?;
    if next_report.as_str() != "report:core:all:2" {
        return Err(format!("report key: {}", next_report.as_str()).into());
    }
    let previous_bests = bests_export_key("core", "2027", "all", &[], &previous)?;
    if previous_bests == bests_export_key("core", "2027", "all", &[], &next)? {
        return Err(format!("bests key: {}", previous_bests.as_str()).into());
    }
    let next_bests = bests_export_key("core", "2027", "all", &[], &next)?;
    if next_bests.as_str() != "bests:core:2027:all:all:2" {
        return Err(format!("bests key: {}", next_bests.as_str()).into());
    }
    let previous_consolidate = consolidate_export_key(&previous);
    if previous_consolidate == consolidate_export_key(&next) {
        return Err(format!("consolidate key: {}", previous_consolidate.as_str()).into());
    }
    let next_consolidate = consolidate_export_key(&next);
    if next_consolidate.as_str() != "consolidate:2" {
        return Err(format!("consolidate key: {}", next_consolidate.as_str()).into());
    }
    Ok(())
}

#[test]
fn export_generations_are_bounded_to_key_safe_text() -> TestResult {
    if ExportGeneration::parse("").is_ok() {
        return Err("an empty export generation is refused".into());
    }
    let too_long = "9".repeat(33);
    if ExportGeneration::parse(&too_long).is_ok() {
        return Err("a 33 byte export generation is refused".into());
    }
    let ceiling = "9".repeat(32);
    if ExportGeneration::parse(&ceiling).is_err() {
        return Err("a 32 byte export generation is selectable".into());
    }
    for refused in ["a/b", "a b", "a:b", "gén", "a+b", "a*b"] {
        if ExportGeneration::parse(refused).is_ok() {
            return Err(format!("{refused} must not become an export generation").into());
        }
    }
    for allowed in ["1", "2", "2026-10-10", "run_7.final", "abc-def_1.2"] {
        if ExportGeneration::parse(allowed).is_err() {
            return Err(format!("{allowed} must stay a selectable export generation").into());
        }
    }
    Ok(())
}

#[test]
fn workbook_keys_follow_the_selected_generation() -> TestResult {
    let request = WorkbookRequest {
        grad_year: Some(2027),
        limit: Some(50),
        scope: Some("core".to_string()),
        out: Some("/tmp/root-a".to_string()),
        school_year: Some(2026),
        jurisdictions: Vec::new(),
    };
    let legacy = ExportGeneration::default_generation();
    let workbook_legacy = workbook_request_key(&request, &legacy)?;
    if workbook_legacy.as_str() != "workbook:2027:core:50:all:/tmp/root-a:2026:1" {
        return Err(format!("workbook key: {}", workbook_legacy.as_str()).into());
    }
    let next = ExportGeneration::parse("2")?;
    let workbook_next = workbook_request_key(&request, &next)?;
    if workbook_next.as_str() != "workbook:2027:core:50:all:/tmp/root-a:2026:2" {
        return Err(format!("workbook key: {}", workbook_next.as_str()).into());
    }
    if workbook_legacy == workbook_next {
        return Err("a fresh generation must not reattach the legacy workflow".into());
    }
    Ok(())
}

#[test]
fn the_generation_resolver_defaults_without_a_flag() -> TestResult {
    let resolved = ExportGeneration::resolve(None)?;
    if resolved.as_str() != DEFAULT_GENERATION {
        return Err(format!("default generation: {}", resolved.as_str()).into());
    }
    let requested = ExportGeneration::resolve(Some("7"))?;
    if requested.as_str() != "7" {
        return Err(format!("requested generation: {}", requested.as_str()).into());
    }
    if ExportGeneration::resolve(Some("bad/generation")).is_ok() {
        return Err("a slash must not become an export generation".into());
    }
    Ok(())
}

#[test]
fn export_keys_refuse_colon_parts_on_every_builder() -> TestResult {
    let generation = ExportGeneration::default_generation();
    check!(
        report_export_key("co:re", &[], &generation).is_err(),
        "a colon scope must not shift the report key segments"
    );
    check!(
        report_export_key("", &[], &generation).is_err(),
        "an empty scope must not collapse the report key grammar"
    );
    check!(
        bests_export_key("a:ll", "2027", "50", &[], &generation).is_err(),
        "a colon scope must not shift the bests key segments"
    );
    check!(
        bests_export_key("all", "20:27", "50", &[], &generation).is_err(),
        "a colon year must not shift the bests key segments"
    );
    check!(
        bests_export_key("all", "2027", "5:0", &[], &generation).is_err(),
        "a colon limit must not shift the bests key segments"
    );
    check!(
        bests_export_key("all", "2027", "", &[], &generation).is_err(),
        "an empty limit must not collapse the bests key grammar"
    );
    check!(
        bests_export_key("", "2027", "50", &[], &generation).is_err(),
        "an empty scope must not collapse the bests key grammar"
    );
    check!(
        bests_export_key("all", "", "50", &[], &generation).is_err(),
        "an empty year must not collapse the bests key grammar"
    );
    Ok(())
}

#[test]
fn offline_routes_refuse_an_explicit_generation() -> TestResult {
    if reject_offline_generation(None).is_err() {
        return Err("an offline route must accept an unrequested generation".into());
    }
    let refused = reject_offline_generation(Some("2"));
    check!(
        refused.is_err(),
        "an offline route must not silently ignore a requested generation"
    );
    Ok(())
}
