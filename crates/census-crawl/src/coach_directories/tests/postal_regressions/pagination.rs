use super::*;

fn directory(
    change: impl FnOnce(&mut serde_json::Value),
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_json::from_slice(DIRECTORY)?;
    change(&mut value);
    Ok(serde_json::to_vec(&value)?)
}

#[test]
fn a_repeated_directory_page_cannot_certify_the_unread_page() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = directory(|page| page["totalPages"] = 2.into())?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            seed(&run.fetcher, &directory_page_url("NCHSAA", 2), &body)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 1));
            check!(report
                .notes
                .iter()
                .any(|note| note.contains("requested page 2, received 1")));
            check!(eq; run.school()?.postal_addresses.len(), 2);
            check!(eq; run.coaches()?.len(), 16);
            Ok(())
        })
}

#[test]
fn the_directory_page_budget_is_disclosed_without_discarding_collected_school_facts() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = directory(|page| page["totalPages"] = 65.into())?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            for page in 2..=64 {
                let empty = serde_json::to_vec(&serde_json::json!({
                    "currentPage": page, "totalPages": 65, "totalResults": 452, "results": []
                }))?;
                seed(&run.fetcher, &directory_page_url("NCHSAA", page), &empty)?;
            }
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 1));
            check!(report
                .notes
                .iter()
                .any(|note| note.contains("more directory pages than the 64-page walk reads")));
            check!(eq; run.coaches()?.len(), 16);
            check!(eq; run.school()?.postal_addresses.len(), 2);
            Ok(())
        })
}

#[test]
fn a_truncated_final_directory_page_keeps_valid_contacts_but_reports_the_missing_population(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = directory(|page| page["totalResults"] = 453.into())?;
            let run = FixtureRun::new(&body, SUMMARY)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 1));
            check!(report
                .notes
                .iter()
                .any(|note| note.contains("read 452 of 453 published rows")));
            check!(eq; run.coaches()?.len(), 16);
            check!(eq; run.school()?.postal_addresses.len(), 2);
            Ok(())
        })
}

#[test]
fn a_directory_or_summary_above_the_row_budget_is_an_explicit_refusal() -> TestResult {
    let body =
        serde_json::to_vec(&serde_json::json!({"results": vec![serde_json::json!({}); 20_001]}))?;
    match super::super::super::parse::parse_directory(&body) {
        Err(crate::CrawlError::Schema { .. }) => {}
        other => return Err(format!("unexpected bounded directory outcome: {other:?}").into()),
    }
    for field in ["staff", "teams"] {
        let body = changed_summary(|summary| {
            summary[field] = serde_json::json!(vec![serde_json::json!({}); 20_001])
        })?;
        match super::super::super::parse::parse_summary(&body) {
            Err(crate::CrawlError::Schema { .. }) => {}
            other => return Err(format!("unexpected bounded summary outcome: {other:?}").into()),
        }
    }
    Ok(())
}
