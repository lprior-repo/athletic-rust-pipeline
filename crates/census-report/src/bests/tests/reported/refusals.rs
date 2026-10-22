use super::*;
use crate::report::ReportError;

#[test]
fn reported_uses_exact_canonical_parents_without_alias_or_name_joins() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track100m)?;
    dataset.lineage.policy_revision = 21;
    let mut unknown = reported(
        &dataset,
        "reported-unknown",
        Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
    );
    let canonical = unknown.athlete.clone();
    unknown.athlete = Id::mint("ath", &["reported-unknown-parent"]);
    dataset
        .canonical_aliases
        .insert(unknown.athlete.as_str().into(), canonical.as_str().into());
    dataset.performances = vec![unknown];
    check!(groups(&dataset)?.is_empty());
    let mut other = dataset.athletes.first().ok_or("canonical athlete")?.clone();
    other.id = Id::mint("ath", &["reported-same-name-parent"]);
    dataset.athletes.push(other.clone());
    let mut first = dataset.performances.first().ok_or("reported row")?.clone();
    first.athlete = canonical.clone();
    let mut second = first.clone();
    second.id = Id::mint("perf", &["reported-other-owner"]);
    second.athlete = other.id.clone();
    dataset.performances = vec![first, second];
    let actual = groups(&dataset)?;
    check!(eq; actual.len(), 2);
    check!(actual.iter().any(|group| group.key.athlete_id == canonical));
    check!(actual.iter().any(|group| group.key.athlete_id == other.id));
    Ok(())
}

#[test]
fn reported_checks_dataset_identity_conflicts_before_subset_filtering() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track100m)?;
    dataset.lineage.policy_revision = 21;
    let first = reported(
        &dataset,
        "reported-identity",
        Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
    );
    let mut contradiction = first.clone();
    contradiction.athlete = Id::mint("ath", &["reported-incompatible-owner"]);
    dataset.performances = vec![first, contradiction];
    let admission = EventAdmission::of(&dataset)?;
    let first = dataset.performances.first().ok_or("first identity row")?;
    let error = reported_best_groups(&dataset, &admission, &[first])
        .err()
        .ok_or("identity contradiction accepted")?;
    check!(error
        .to_string()
        .contains("incompatible retained identity or context"));
    Ok(())
}

#[test]
fn reported_rejects_foreign_admission_and_external_performance_clones() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track100m)?;
    dataset.lineage.policy_revision = 21;
    let performance = reported(
        &dataset,
        "reported-exact-input",
        Mark::TimeSeconds(ExactSeconds::parse("10.50")?),
    );
    dataset.performances = vec![performance];
    let other = selection_dataset(EventKind::LongJump)?;
    let foreign_admission = EventAdmission::of(&other)?;
    let retained = dataset.performances.first().ok_or("retained row")?;
    check!(matches!(
        reported_best_groups(&dataset, &foreign_admission, &[retained]),
        Err(ReportError::Invariant { .. })
    ));
    let external = (*retained).clone();
    let admission = EventAdmission::of(&dataset)?;
    let error = reported_best_groups(&dataset, &admission, &[&external])
        .err()
        .ok_or("external clone accepted")?;
    check!(error
        .to_string()
        .contains("not a row of the admitted dataset"));
    check!(eq; reported_best_groups(&dataset, &admission, &[retained])?.len(), 1);
    Ok(())
}
