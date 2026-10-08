use super::*;
use census_domain::model::{CompetitionLevel, SourceIdentity, SourceNamespace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const HARVEST: &str = include_str!("../../tests/fixtures/athleticlive/meets-sample.csv");

#[test]
fn parses_harvest_rows_and_keeps_only_known_states() -> TestResult {
    let rows = parse_meets_csv(HARVEST)?;
    if rows.len() < 4 {
        return Err(format!(
            "fixture should carry several rows: length {} < 4",
            rows.len()
        )
        .into());
    }
    if !rows.iter().all(|r| !r.name.is_empty()) {
        return Err("every kept row has a name".into());
    }
    if !rows
        .iter()
        .any(|r| r.state_code == UsJurisdiction::Illinois)
    {
        return Err("harvest must contain Illinois".into());
    }
    if !rows.iter().any(|r| r.athleticnet_meet_id.is_some()) {
        return Err("the harvest carries Athletic.net meet ids".into());
    }
    Ok(())
}

#[test]
fn quoted_fields_do_not_break_column_alignment() -> TestResult {
    let csv = "tenant,athleticlive_meet_id,athleticnet_meet_id,name,city_state,state,start,end,has_results,timer_credit\n\
               palatine,55274,259955,\"4th Annual St. Pius X, Knights Classic\",\"Lombard, IL\",Illinois,2025-08-16T04:00:00Z,,True,\"Timed by <a href=\"\"x\"\">Palatine Pack</a>\"\n";
    let rows = parse_meets_csv(csv)?;
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].name, "4th Annual St. Pius X, Knights Classic");
    check!(eq; rows[0].city_state.as_deref(), Some("Lombard, IL"));
    check!(eq; rows[0].state_code, UsJurisdiction::Illinois);
    check!(eq; rows[0].athleticnet_meet_id.as_deref(), Some("259955"));
    check!(rows[0].has_results);
    Ok(())
}

#[test]
fn missing_required_column_is_an_error_not_a_panic() -> TestResult {
    match parse_meets_csv("tenant,name,state,start\nx,Meet,Illinois,2025-08-16T04:00:00Z\n") {
        Err(err) => {
            if !err.to_string().contains("athleticlive_meet_id") {
                return Err(format!(
                    "missing required column must name athleticlive_meet_id: {err}"
                )
                .into());
            }
        }
        Ok(_) => return Err("athleticlive_meet_id is required".into()),
    }
    Ok(())
}

#[test]
fn corrupt_years_are_flagged_not_silently_kept() -> TestResult {
    for (date, expected) in [
        ("2026-05-29", false),
        ("2015-01-01", false),
        ("2222-08-23", true),
        ("not-a-date", true),
    ] {
        let actual = implausible_year(date);
        if actual != expected {
            return Err(format!(
                "implausible_year({date:?}): left: {actual:?}, right: {expected:?}"
            )
            .into());
        }
    }
    let rows = parse_meets_csv(HARVEST)?;
    if !rows.iter().all(|r| !implausible_year(&r.start)) {
        return Err("every harvest row must have a plausible start year".into());
    }
    Ok(())
}

#[test]
fn level_inference_only_fires_on_explicit_markers() {
    assert_eq!(
        infer_level("WIAA State Championships"),
        CompetitionLevel::State
    );
    assert_eq!(infer_level("D3 Sectional #3"), CompetitionLevel::Sectional);
    assert_eq!(
        infer_level("Big Rivers Conference Meet"),
        CompetitionLevel::Conference
    );
    assert_eq!(
        infer_level("Knights Chicagoland Classic"),
        CompetitionLevel::Invitational
    );
    assert_eq!(infer_level("Weekend Race #4"), CompetitionLevel::Unknown);
}

#[test]
fn tenants_publishing_one_meet_merge_into_one_canonical_meet() -> TestResult {
    let rows = parse_meets_csv(HARVEST)?;
    let meets = build_meets(&rows, "2026-09-20", "athleticlive_meets_csv");
    let classic = meets
        .iter()
        .find(|m| m.name.contains("Knights Chicagoland"))
        .ok_or("fixture meet present")?;
    let timers: Vec<&SourceIdentity> = classic
        .source_identities
        .iter()
        .filter(|i| matches!(i.namespace, SourceNamespace::TimerMeet { .. }))
        .collect();
    check!(
        timers.len() >= 2,
        "both tenants are recorded on one canonical meet"
    );
    check!(eq; timers[0].id, "55274");
    let an: Vec<&SourceIdentity> = classic
        .source_identities
        .iter()
        .filter(|identity| identity.namespace == SourceNamespace::athletic_net("meet"))
        .collect();
    check!(eq; an.len(), 1, "the Athletic.net meet id is deduplicated");
    check!(eq; an[0].id, "259955");
    check!(eq;
        an[0].url.as_deref(),
        Some("https://www.athletic.net/TrackAndField/meet/259955/info")
    );
    check!(eq; classic.location.as_deref(), Some("Lombard, IL"));
    check!(eq; classic.date, "2025-08-16");
    check!(eq; meets.iter().filter(|m| m.id == classic.id).count(), 1);
    Ok(())
}
