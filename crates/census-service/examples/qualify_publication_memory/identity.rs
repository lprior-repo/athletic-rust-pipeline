use super::config::DAY;
use anyhow::{ensure, Result};
use census_domain::model::*;
use census_store::{Store, Table};

pub fn accept(store: &Store, members: &[CanonicalAthlete; 2]) -> Result<()> {
    let index = store.athlete_identity_index()?;
    let ids = members
        .iter()
        .map(|athlete| athlete.id.cast())
        .collect::<Vec<_>>();
    let subject = "Synthetic memory fixture alias";
    let detail = "Synthetic shared provider ID with two published spellings";
    let evidence = index.case_evidence(subject, detail, &ids)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members[0].id.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = ids;
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: detail.into(),
        reviewer: "synthetic fixture".into(),
        observed_at: DAY.into(),
        member_ids: case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;
    let cases = [case];
    let verdicts = [verdict];
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let decisions = builder
        .reviewed_applications(DAY)
        .map(|(_, result)| match result? {
            IdentityApplication::Accepted(decision) => Ok(decision),
            IdentityApplication::Retained(issue) => {
                anyhow::bail!("synthetic alias refused: {issue:?}")
            }
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(decisions.len() == 1);
    ensure!(store.apply_identity_decisions(&decisions)? == 1);
    Ok(())
}
