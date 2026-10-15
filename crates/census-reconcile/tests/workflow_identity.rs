mod common;

use census_domain::model::Revision;
use census_domain::UsJurisdiction;
use census_reconcile::identity::{admitted_scope, WorkflowIdentity, MAX_IDENTITY_BYTES};
use common::{season, TestResult};

#[test]
fn national_identity_is_generation_not_calendar() -> TestResult {
    let scope = [UsJurisdiction::Wisconsin];
    let first = WorkflowIdentity::national(season()?, Revision(1), &scope);
    let again = WorkflowIdentity::national(season()?, Revision(1), &scope);
    if first.as_str() != "national:2026-27:8ee4f7269711dd03:1" {
        return Err(format!("national identity: {}", first.as_str()).into());
    }
    if first != again {
        return Err("the same generation and scope mint the same identity".into());
    }
    let separators = first.as_str().matches(':').count();
    if separators != 3 {
        return Err(format!("identity separators: {separators}").into());
    }
    if Revision(7).get() != 7 {
        return Err(format!("revision: {}", Revision(7).get()).into());
    }
    if Revision(7).to_string() != "7" {
        return Err(format!("revision text: {}", Revision(7)).into());
    }
    Ok(())
}

#[test]
fn revision_and_scope_change_the_run() -> TestResult {
    let scope = [UsJurisdiction::Wisconsin];
    let first = WorkflowIdentity::national(season()?, Revision(1), &scope);
    if first == WorkflowIdentity::national(season()?, Revision(2), &scope) {
        return Err("a revision change must change the identity".into());
    }
    if first == WorkflowIdentity::national(season()?, Revision(1), &[UsJurisdiction::Iowa]) {
        return Err("a scope change must change the identity".into());
    }
    let jurisdiction =
        WorkflowIdentity::jurisdiction(UsJurisdiction::Wisconsin, season()?, Revision(1));
    if jurisdiction.as_str() != "jurisdiction:WI:2026-27:1" {
        return Err(format!("jurisdiction identity: {}", jurisdiction.as_str()).into());
    }
    Ok(())
}

#[test]
fn scope_set_semantics_hold_over_order_and_duplication() -> TestResult {
    let forward = [UsJurisdiction::Iowa, UsJurisdiction::Wisconsin];
    let backward = [UsJurisdiction::Wisconsin, UsJurisdiction::Iowa];
    let forward_identity = WorkflowIdentity::national(season()?, Revision(1), &forward);
    let backward_identity = WorkflowIdentity::national(season()?, Revision(1), &backward);
    if forward_identity != backward_identity {
        return Err("scope order must not change the identity".into());
    }
    let repeated = [
        UsJurisdiction::Iowa,
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
    ];
    let duplicated = WorkflowIdentity::national(season()?, Revision(1), &repeated);
    if forward_identity != duplicated {
        return Err("a duplicated scope member must not change the identity".into());
    }
    let empty = WorkflowIdentity::national(season()?, Revision(1), &[]);
    let census = WorkflowIdentity::national(season()?, Revision(1), &UsJurisdiction::CENSUS_SCOPE);
    if empty.as_str() != census.as_str() {
        return Err(format!("empty scope: {}", empty.as_str()).into());
    }
    if admitted_scope(&[]) != UsJurisdiction::CENSUS_SCOPE.to_vec() {
        return Err("the empty scope admits the whole census scope".into());
    }
    let admitted = admitted_scope(&forward);
    if admitted != vec![UsJurisdiction::Iowa, UsJurisdiction::Wisconsin] {
        return Err(format!("admitted scope: {admitted:?}").into());
    }
    Ok(())
}

#[test]
fn jurisdiction_identities_fit_ceiling() -> TestResult {
    for jurisdiction in UsJurisdiction::ALL {
        let identity = WorkflowIdentity::jurisdiction(jurisdiction, season()?, Revision(u32::MAX));
        if identity.as_str().len() > MAX_IDENTITY_BYTES {
            return Err(format!("{} exceeds the ceiling", jurisdiction.code()).into());
        }
    }
    let full =
        WorkflowIdentity::national(season()?, Revision(u32::MAX), &UsJurisdiction::CENSUS_SCOPE);
    if full.as_str().len() > MAX_IDENTITY_BYTES {
        return Err(format!("national identity length: {}", full.as_str().len()).into());
    }
    Ok(())
}
