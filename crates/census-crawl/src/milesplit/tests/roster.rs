use super::*;

#[test]
fn raw_row_that_does_not_fit_the_column_map_is_reported() -> TestResult {
    let mutated: String = OH_RAW
        .lines()
        .map(|line| match line.contains("Jeydyn Fields") {
            true => {
                let (head, tail) = line.split_at(74);
                format!("{head}X{tail}\n")
            }
            false => format!("{line}\n"),
        })
        .collect();
    let page = parse_raw(&mutated, OH_RAW_URL)?;
    check!(eq; page.meet.rows_parsed, 79);
    check!(eq; page.skipped.len(), 1);
    check!(eq; page.meet.rows_skipped, 1);
    Ok(())
}

#[test]
fn result_set_urls_are_checked_and_disallowed_routes_are_never_built() -> TestResult {
    let reference = ResultSetRef::parse(OH_RAW_URL).ok_or("Ohio result set")?;
    check!(eq; reference.meet_id, "770621");
    check!(eq; reference.rsid, "1321880");
    check!(eq; reference.site.code(), "OH");
    check!(eq; reference.url, OH_RAW_URL);
    for rejected in [
        "https://oh.milesplit.com/api/v1/meets/770621/performances",
        "https://api.prod.milesplit.com/v1/meets/770621/performances",
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/formatted",
        "https://www.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw",
        "https://oh.milesplit.com/teams",
    ] {
        check!(ResultSetRef::parse(rejected).is_none(), "accepted {rejected}");
    }
    for jurisdiction in UsJurisdiction::ALL {
        let site = Site::for_jurisdiction(jurisdiction);
        let url = site.teams_url();
        check!(
            url.starts_with(&format!(
                "https://{}.milesplit.com/teams",
                jurisdiction.code().to_ascii_lowercase()
            )) && !url.contains("/api/"),
            "url: {url}"
        );
    }
    Ok(())
}

#[test]
fn a_results_page_lists_the_result_files_the_raw_route_serves() -> TestResult {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, OH_MEET_RESULTS)?;
    check!(eq; files.len(), 1);
    check!(eq; files[0].id, 1321880);
    check!(eq; files[0].name, "Results");
    check!(eq; files[0].is_meet_pro, 0);
    Ok(())
}

#[test]
fn a_page_without_a_file_list_is_a_schema_mismatch() -> TestResult {
    let error = match parse_meet_result_files(
        "https://www.milesplit.com/meets/1-a/results",
        "<html><body>no result files here</body></html>",
    ) {
        Err(error) => error,
        Ok(_) => return Err("no file list is a mismatch".into()),
    };
    check!(matches!(error, CrawlError::Schema { .. }), "{error:?}");
    Ok(())
}

#[test]
fn an_empty_file_list_is_a_state_not_a_failure() -> TestResult {
    let files = parse_meet_result_files(OH_MEET_RESULTS_URL, "let meetResultFiles = [];")?;
    check!(files.is_empty());
    Ok(())
}
