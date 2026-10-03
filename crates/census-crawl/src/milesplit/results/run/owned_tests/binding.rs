use super::*;
use crate::net::FetchOutcome;
use census_domain::model::Sport;

mod captured_metadata;
use captured_metadata::assert_archived_metadata;

fn capture(url: &str, body: &[u8]) -> FetchOutcome {
    FetchOutcome {
        url: url.into(),
        response_url: None,
        method: "GET".into(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-01T23:44:16Z".into(),
        from_cache: true,
        content_type: Some("text/html".into()),
        body: body.to_vec(),
    }
}

#[test]
fn both_authentic_troy_raw_documents_publish_march_outdoor_metadata_for_school_year_2025(
) -> TestResult {
    for (id, body) in [("1266814", FEMALE_RAW), ("1266815", MALE_RAW)] {
        let reference = ResultSetRef::parse(&format!(
            "https://al.milesplit.com/meets/725218/results/{id}/raw"
        ))
        .ok_or("source set")?;
        let page =
            super::super::metadata::parse_capture(&capture(&reference.url, body), &reference)?;
        check!(eq; page.meet.name, "Troy Invitational #2");
        check!(eq; page.meet.date, "2026-03-27");
        check!(eq; page.meet.end_date.as_deref(), Some("2026-03-27"));
        check!(eq; page.sport, Some(Sport::OutdoorTrack));
        check!(eq; page.region.as_deref(), Some("AL"));
        check!(eq;
            page.school_year,
            SchoolYear::new(2025).ok_or("published season")?
        );
    }
    Ok(())
}

#[test]
fn foreign_meet_result_set_or_canonical_document_metadata_never_projects_owned_troy_rows(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let female = std::str::from_utf8(FEMALE_RAW)?;
    let foreign_meet = female.replace(
        "725218-troy-invitational-2-2026",
        "770621-beaver-eastern-invite",
    );
    let foreign_canonical = female.replace(
        "href=\"https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266814/raw\"",
        "href=\"https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results/1266815/raw\"",
    );
    let missing_owner = female.replace("\"url\":", "\"unowned_url\":");
    for body in [
        foreign_meet.as_bytes(),
        MALE_RAW,
        foreign_canonical.as_bytes(),
        missing_owner.as_bytes(),
    ] {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        seed(&fetcher, &reference.url, body)?;
        let report =
            crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &options(&reference))
                .await
                ?;
        check!(eq; (report.rows, report.errors), (0, 1));
        check!(eq;
            store
                .scan::<CanonicalPerformance>(Table::Performances)
                ?,
            vec![]
        );
        check!(eq;
            store
                .scan::<CanonicalAthlete>(Table::Athletes)
                ?,
            vec![]
        );
        check!(eq;
            store
                .walk_table(Table::SourceObservations)
                ?
                .rows,
            0
        );
        let payloads = store
            .journal_payloads(super::super::super::RESULT_SET_PHASE)
            ?;
        check!(eq; payloads.len(), 1);
        check!(eq; payloads[0]["disposition"], "metadata_unavailable");
        check!(eq; payloads[0]["meet"], reference.meet_id);
        check!(eq; payloads[0]["rsid"], reference.rsid);
        check!(eq;
            payloads[0]["raw_metadata_capture"]["content_digest"],
            crate::net::cache::content_digest(body)
        );
        check!(eq; payloads[0]["raw_metadata_capture"]["url"], reference.url);
        check!(eq;
            payloads[0]["raw_metadata_capture"]["response_url"],
            serde_json::Value::Null
        );
        assert_archived_metadata(&store, body)?;
        check!(store
            .journal_payloads(crate::milesplit::OWNED_MEET_PHASE)
            ?
            .iter()
            .any(|row| row["result_id"] == 201782263));
    }
    Ok(())
    })
}

#[test]
fn foreign_captured_or_requested_raw_urls_cannot_bind_a_matching_published_troy_document(
) -> TestResult {
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("Troy reference")?;
    for url in [
        "https://al.milesplit.com/meets/770621/results/1266814/raw",
        "https://al.milesplit.com/meets/725218/results/1266815/raw",
        "https://oh.milesplit.com/meets/725218/results/1266814/raw",
        "https://al.milesplit.com/meets/0725218/results/1266814/raw",
        "https://al.milesplit.com/meets/725218/results/1266814/raw?meet=770621",
    ] {
        match super::super::metadata::parse_capture(&capture(url, FEMALE_RAW), &reference) {
            Err(crate::CrawlError::Schema { url, .. }) => {
                check!(eq; url, reference.url);
            }
            outcome => {
                return Err(
                    format!("foreign capture metadata must remain unresolved: {outcome:?}").into(),
                )
            }
        }
        let mut rebound = reference.clone();
        rebound.url = url.into();
        match super::super::metadata::parse_capture(&capture(&reference.url, FEMALE_RAW), &rebound)
        {
            Err(crate::CrawlError::Schema { url, .. }) => {
                check!(eq; url, rebound.url);
            }
            outcome => {
                return Err(
                    format!("foreign request metadata must remain unresolved: {outcome:?}").into(),
                )
            }
        }
    }
    Ok(())
}

#[test]
fn contradictory_observed_raw_response_cannot_bind_a_matching_published_document() -> TestResult {
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("Troy reference")?;
    for response in [
        "https://al.milesplit.com/meets/770621/results/1266814/raw",
        "https://al.milesplit.com/meets/725218/results/1266815/raw",
        "https://oh.milesplit.com/meets/725218/results/1266814/raw",
    ] {
        let mut captured = capture(&reference.url, FEMALE_RAW);
        captured.response_url = Some(response.into());
        match super::super::metadata::parse_capture(&captured, &reference) {
            Err(crate::CrawlError::Schema { url, .. }) => {
                check!(eq; url, reference.url);
            }
            outcome => {
                return Err(
                    format!("foreign observed owner must remain unresolved: {outcome:?}").into(),
                )
            }
        }
    }
    Ok(())
}
