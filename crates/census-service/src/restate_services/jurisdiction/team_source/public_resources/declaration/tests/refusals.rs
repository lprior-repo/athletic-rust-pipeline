use super::fixture::{owner, request, Fixture, TestResult};
use crate::restate_services::plan::RefusalKind;
use crate::restate_services::wire::{RefusedSource, SatisfiedSource};
use census_domain::model::*;
use census_domain::UsJurisdiction;

type Mutation = fn(&mut Fixture) -> TestResult;

#[test]
fn malformed_or_unselected_retained_owner_facts_refuse_without_input_changes() -> TestResult {
    let mutations: [Mutation; 10] = [
        |f| {
            f.first_mut()?.state.identity.push_str(":foreign");
            Ok(())
        },
        |f| {
            f.first_mut()?.state.history_window = None;
            Ok(())
        },
        |f| {
            f.first_mut()?.state.history_window = Some(HistoryWindow::cohort("2026-10-20")?);
            Ok(())
        },
        |f| {
            f.first_mut()?.state.plan = None;
            Ok(())
        },
        |f| {
            f.plan_mut()?.sweepable.retain(|slug| slug != "milesplit");
            Ok(())
        },
        |f| {
            f.plan_mut()?.sweepable.push("milesplit".to_string());
            Ok(())
        },
        |f| {
            f.plan_mut()?.refused.push(RefusedSource {
                slug: "milesplit".to_string(),
                reason: "controlled refusal".to_string(),
                kind: RefusalKind::AccessBlocked,
            });
            Ok(())
        },
        |f| {
            f.plan_mut()?.satisfied.push(SatisfiedSource {
                slug: "milesplit".to_string(),
                evidence_digest: serialized_digest(&"controlled source evidence")?,
            });
            Ok(())
        },
        |f| {
            f.plan_mut()?.fingerprint = "not-a-digest".to_string();
            Ok(())
        },
        |f| {
            f.plan_mut()?.fingerprint = "A".repeat(64);
            Ok(())
        },
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new()?;
        mutate(&mut fixture)?;
        fixture.refused_without_input_changes()?;
    }
    Ok(())
}

#[test]
fn missing_duplicate_and_foreign_owner_inventories_are_refused() -> TestResult {
    let mutations: [Mutation; 4] = [
        |f| {
            f.facts.pop().ok_or("owner")?;
            Ok(())
        },
        |f| {
            let site = f.first_mut()?.site;
            let duplicate = owner(&f.bound, site)?;
            f.facts.pop().ok_or("owner")?;
            f.facts.push(duplicate);
            Ok(())
        },
        |f| {
            let foreign = owner(&f.bound, UsJurisdiction::Ohio)?;
            f.facts.pop().ok_or("owner")?;
            f.facts.push(foreign);
            Ok(())
        },
        |f| {
            f.facts.clear();
            Ok(())
        },
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new()?;
        mutate(&mut fixture)?;
        fixture.refused_without_input_changes()?;
    }
    Ok(())
}

#[test]
fn foreign_bound_scope_run_and_selected_store_are_refused() -> TestResult {
    let mutations: [Mutation; 9] = [
        |f| {
            f.bound.jurisdictions.pop().ok_or("jurisdiction")?;
            Ok(())
        },
        |f| {
            f.bound.jurisdictions.push(UsJurisdiction::Ohio);
            Ok(())
        },
        |f| {
            let first = f
                .bound
                .jurisdictions
                .first()
                .copied()
                .ok_or("jurisdiction")?;
            f.bound.jurisdictions.pop().ok_or("jurisdiction")?;
            f.bound.jurisdictions.push(first);
            Ok(())
        },
        |f| {
            f.bound.jurisdictions.clear();
            Ok(())
        },
        |f| {
            f.bound.revision = Revision(2);
            Ok(())
        },
        |f| {
            f.bound.season = SchoolYear::new(2027).ok_or("season")?;
            Ok(())
        },
        |f| {
            f.manifest.cohort = GradYear::new(2026).ok_or("cohort")?;
            Ok(())
        },
        |f| {
            f.manifest.store_identity = serialized_digest(&"another controlled store")?;
            Ok(())
        },
        |f| {
            let selection = f.bound.export_job.require_selected_publication();
            f.bound.export_job = census_report::export::ExportJobRequest::SelectedPublicationV1 {
                selection: ProfileSelectionV1::new(
                    StoreIdentity::parse(&serialized_digest(&"foreign selected store")?)?,
                    selection.profile_id().clone(),
                ),
            };
            Ok(())
        },
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new()?;
        mutate(&mut fixture)?;
        fixture.refused_without_input_changes()?;
    }
    Ok(())
}

#[test]
fn legacy_and_changed_root_requests_cannot_authorize_normalized_directory_capture() -> TestResult {
    let mutations: [Mutation; 7] = [
        |f| {
            f.first_mut()?.directory_request = request("https://co.milesplit.com/teams")?;
            Ok(())
        },
        |f| {
            f.first_mut()?.directory_request = request("https://mi.milesplit.com/teams?offset=1")?;
            Ok(())
        },
        |f| {
            f.first_mut()?.directory_request = request("https://mi.milesplit.com/teams/1/roster")?;
            Ok(())
        },
        |f| {
            f.first_mut()?.robots_request = request("https://co.milesplit.com/robots.txt")?;
            Ok(())
        },
        |f| {
            f.first_mut()?.robots_request = request("https://mi.milesplit.com/robots.txt?raw=1")?;
            Ok(())
        },
        |f| {
            f.first_mut()?.directory_request = RequestRepresentationV1::get(
                "https://mi.milesplit.com/teams",
                Vec::new(),
                "Census/service-declaration-test",
            )?;
            Ok(())
        },
        |f| {
            f.first_mut()?.robots_request = RequestRepresentationV1::get(
                "https://mi.milesplit.com/robots.txt",
                Vec::new(),
                "Census/service-declaration-test",
            )?;
            Ok(())
        },
    ];
    for mutate in mutations {
        let mut fixture = Fixture::new()?;
        mutate(&mut fixture)?;
        fixture.refused_without_input_changes()?;
    }
    Ok(())
}
