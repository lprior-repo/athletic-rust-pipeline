use super::*;

#[test]
fn foreign_owned_meet_or_result_set_indices_are_refused_before_any_projection_mutation(
) -> TestResult {
    let body = serde_json::to_vec(&document()?)?;
    let OwnedMeetVerdict::Parsed(page) = parse_owned_meet(&body, 725218) else {
        return Err("source fixture".into());
    };
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("Troy reference")?;
    for (meet, set) in [(770621, 1266814), (725218, 1266815)] {
        let mut foreign = page.clone();
        foreign.rows[1].meet_id = meet;
        foreign.rows[1].result_set_id = set;
        let capture = FetchOutcome {
            url: "https://al.milesplit.com/api/v1/meets/725218/performances".into(),
            response_url: None,
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(&body),
            bytes: body.len(),
            fetched_at: "2026-10-01T23:44:16Z".into(),
            from_cache: true,
            content_type: Some("application/json".into()),
            body: body.clone(),
        };
        let mut accumulated = Accumulator::default();
        let mut stats = Stats::default();
        match absorb_result_set(
            &metadata()?,
            &reference,
            OwnedResultSet {
                capture: &capture,
                page: &foreign,
                indices: &[0, 1, 2],
                performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 22)
                    .ok_or("snapshot date")?,
            },
            &mut RowWriter {
                schools: &ProviderSchools::from_schools(&[
                    school("Spann", "38332"),
                    school("Charles", "4912"),
                ]),
                stats: &mut stats,
                accumulated: &mut accumulated,
            },
        ) {
            Err(crate::CrawlError::Schema { detail, .. }) => {
                check!(eq; detail, "owned row does not match requested meet/result-set");
            }
            outcome => {
                return Err(format!(
                    "foreign owned row cannot mint canonical entities: {outcome:?}"
                )
                .into())
            }
        }
        check!(eq; stats.rows, 0);
        check!(eq; accumulated.meets, HashMap::new());
        check!(eq; accumulated.athletes, HashMap::new());
        check!(eq; accumulated.performances, HashMap::new());
        check!(eq; accumulated.observations, HashMap::new());
    }
    Ok(())
}
