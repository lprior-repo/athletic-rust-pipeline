use super::*;
use crate::restate_services::wire::{RefusedSource, SourcePlan};

#[test]
fn refused_history_sources_are_disposed_without_changing_result_selection(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = JurisdictionState {
        plan: Some(SourcePlan {
            sweepable: vec!["milesplit".into()],
            refused: vec![
                RefusedSource {
                    slug: "athleticlive".into(),
                    reason: "missing producer capture".into(),
                    kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
                },
                RefusedSource {
                    slug: "wi_missing_coach_sources".into(),
                    reason: "supplemental coach obligation, not a registry family".into(),
                    kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
                },
            ],
            fingerprint: String::new(),
        }),
        ..JurisdictionState::default()
    };
    let meets = meet_history_sources(&state).map_err(crate::restate_services::tests::sdk_error)?;
    check!(eq; meets.iter().map(|source| source.slug).collect::<Vec<_>>(),
        vec!["athleticlive", "milesplit"]);
    let results = history_sources(&state).map_err(crate::restate_services::tests::sdk_error)?;
    check!(eq; results.iter().map(|source| source.slug).collect::<Vec<_>>(),
        vec!["milesplit"]);
    check!(eq; state.plan.ok_or("plan retained")?.refused.first()
        .ok_or("refusal retained")?.reason, "missing producer capture");
    Ok(())
}
#[test]
fn delegated_results_reach_terminal_when_acquisition_is_complete(
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::census::{MeetCensus, MeetSourceRows};
    use crate::restate_services::results_arms::outcomes::delegated_source;
    use census_crawl::{CollectionDisposition, UnresolvedCounters};
    let mut outcome = crate::restate_services::results_arms::ResultsStageOutcome {
        required_sources: vec!["wiaa_results".into()],
        per_source: vec![delegated_source("wiaa_results")],
        pending: Vec::new(),
    };
    check!(!outcome.is_terminal());
    let acquired = MeetSourceRows {
        slug: "wiaa_results".into(),
        rows: 42,
        disposition: CollectionDisposition::Complete,
        unresolved: Some(UnresolvedCounters { rows: 0, labels: 0 }),
        withheld: Some(0),
        ..MeetSourceRows::default()
    };
    let meets = Some(&MeetCensus {
        sources: vec![acquired],
        ..MeetCensus::default()
    });
    bind_acquired_results(&mut outcome, meets)
        .map_err(|error| format!("bind failed: {error:?}"))?;
    let wiaa = outcome
        .per_source
        .iter()
        .find(|s| s.slug == "wiaa_results")
        .ok_or("missing wiaa_results")?;
    check!(wiaa.resolution.is_some());
    check!(eq; wiaa.rows, Some(42));
    check!(outcome.is_terminal());
    Ok(())
}
