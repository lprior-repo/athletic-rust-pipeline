use super::*;

fn directory(change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(DIRECTORY).expect("directory capture");
    change(&mut value);
    serde_json::to_vec(&value).expect("changed pagination envelope")
}

#[tokio::test]
async fn a_repeated_directory_page_cannot_certify_the_unread_page() {
    let body = directory(|page| page["totalPages"] = 2.into());
    let run = FixtureRun::new(&body, SUMMARY);
    seed(&run.fetcher, &directory_page_url("NCHSAA", 2), &body);
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (1, 1));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("requested page 2, received 1")));
    assert_eq!(run.school().postal_addresses.len(), 2);
    assert_eq!(run.coaches().len(), 16);
}

#[tokio::test]
async fn the_directory_page_budget_is_disclosed_without_discarding_collected_school_facts() {
    let body = directory(|page| page["totalPages"] = 65.into());
    let run = FixtureRun::new(&body, SUMMARY);
    for page in 2..=64 {
        let empty = serde_json::to_vec(&serde_json::json!({
            "currentPage": page, "totalPages": 65, "totalResults": 452, "results": []
        }))
        .expect("empty bounded directory page");
        seed(&run.fetcher, &directory_page_url("NCHSAA", page), &empty);
    }
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (1, 1));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("more directory pages than the 64-page walk reads")));
    assert_eq!(run.coaches().len(), 16);
    assert_eq!(run.school().postal_addresses.len(), 2);
}

#[tokio::test]
async fn a_truncated_final_directory_page_keeps_valid_contacts_but_reports_the_missing_population()
{
    let body = directory(|page| page["totalResults"] = 453.into());
    let run = FixtureRun::new(&body, SUMMARY);
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (1, 1));
    assert!(report
        .notes
        .iter()
        .any(|note| note.contains("read 452 of 453 published rows")));
    assert_eq!(run.coaches().len(), 16);
    assert_eq!(run.school().postal_addresses.len(), 2);
}

#[test]
fn a_directory_or_summary_above_the_row_budget_is_an_explicit_refusal() {
    let body =
        serde_json::to_vec(&serde_json::json!({"results": vec![serde_json::json!({}); 20_001]}))
            .expect("oversized directory rows");
    match super::super::super::parse::parse_directory(&body) {
        Err(crate::CrawlError::Schema { detail, .. }) => {
            assert_eq!(detail, "directory exceeds 20000 school rows")
        }
        other => panic!("unexpected bounded directory outcome: {other:?}"),
    }
    for field in ["staff", "teams"] {
        let body = changed_summary(|summary| {
            summary[field] = serde_json::json!(vec![serde_json::json!({}); 20_001])
        });
        match super::super::super::parse::parse_summary(&body) {
            Err(crate::CrawlError::Schema { detail, .. }) => {
                assert_eq!(detail, "summary exceeds 20000 staff or team rows")
            }
            other => panic!("unexpected bounded summary outcome: {other:?}"),
        }
    }
}
