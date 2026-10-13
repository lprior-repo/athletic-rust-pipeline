use super::*;
use census_domain::model::{
    CoachLaneEvidence, CoachLaneMailbox, CoachLaneObligation, CoachLaneResolution, CoachLaneRole,
    CoachLaneStage, CoachLaneTenure, CoverageRow, CoverageScope, PendingCoachLane, SchoolYear,
    SourceAccessCondition,
};
use census_domain::UsJurisdiction;

fn published(
    scope: CoverageScope,
    subject: &str,
    metrics: &[(&str, u64)],
) -> TestResult<CoverageRow> {
    let mut row = CoverageRow::new(scope, subject);
    for (key, value) in metrics {
        row = row.with(key, *value);
    }
    Ok(row)
}

fn lane(
    jurisdiction: UsJurisdiction,
    arm: &str,
    observed_on: &str,
) -> TestResult<CoachLaneObligation> {
    Ok(CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction,
        season: SchoolYear::new(2026).ok_or("invalid season")?,
        revision: 1,
        arm: arm.to_string(),
        arm_basis: "state association directory".to_string(),
        observed_on: observed_on.to_string(),
    })?)
}

#[test]
fn source_population_reconciles_from_published_rows() -> TestResult {
    let (_dir, store) = fixture_store()?;
    store.replace_many(
        Table::Coverage,
        &[
            published(
                CoverageScope::Jurisdiction,
                "WI",
                &[
                    ("registered", 12),
                    ("attempted", 9),
                    ("captured", 7),
                    ("parsed", 6),
                    ("accepted", 5),
                    ("withheld", 2),
                ],
            )?,
            published(
                CoverageScope::Jurisdiction,
                "MN",
                &[
                    ("registered", 8),
                    ("attempted", 6),
                    ("captured", 6),
                    ("parsed", 5),
                    ("accepted", 4),
                    ("withheld", 1),
                ],
            )?,
        ],
    )?;

    let report = coverage_of(&store, Some(2027))?;
    let population = report.source_population;
    check!(eq; population.registered, Some(20));
    check!(eq; population.applicable, None);
    check!(eq; population.attempted, Some(15));
    check!(eq; population.captured, Some(13));
    check!(eq; population.parsed, Some(11));
    check!(eq; population.accepted, Some(9));
    check!(eq; population.withheld, Some(3));
    report.reconcile()?;

    let wi = row(&report, "WI")?;
    check!(eq; wi.source_population.registered, Some(12));
    check!(eq; wi.source_population.accepted, Some(5));
    let mn = row(&report, "MN")?;
    check!(eq; mn.source_population.withheld, Some(1));
    Ok(())
}

#[test]
fn a_partially_declared_stage_is_unmeasured_rather_than_zero() -> TestResult {
    let (_dir, store) = fixture_store()?;
    store.replace_many(
        Table::Coverage,
        &[
            published(
                CoverageScope::Jurisdiction,
                "WI",
                &[("registered", 4), ("attempted", 3)],
            )?,
            published(CoverageScope::Jurisdiction, "MN", &[("registered", 6)])?,
        ],
    )?;

    let report = coverage_of(&store, Some(2027))?;
    check!(eq; report.source_population.registered, Some(10));
    check!(eq; report.source_population.attempted, None);
    check!(eq; report.source_population.accepted, None);
    report.reconcile()?;
    let mn = row(&report, "MN")?;
    check!(eq; mn.source_population.registered, Some(6));
    check!(eq; mn.source_population.attempted, None);
    Ok(())
}

#[test]
fn contact_states_are_distinct() -> TestResult {
    let (_dir, store) = fixture_store()?;

    let armed = lane(UsJurisdiction::Wisconsin, "state-association", "2026-10-01")?;
    let armed = armed.arm(
        CoachLaneEvidence::new(
            3,
            2,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "2026-10-02",
        )?,
        CoachLaneResolution::resolved(
            CoachLaneRole::Published,
            CoachLaneTenure::Current,
            CoachLaneMailbox::Published,
            "2026-10-02",
            30,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )?,
    )?;

    let blocked = lane(UsJurisdiction::Minnesota, "state-association", "2026-10-01")?;
    let blocked = blocked.record(
        CoachLaneStage::Blocked,
        None,
        CoachLaneResolution::unresolved("2026-10-02"),
    )?;

    let failed = lane(UsJurisdiction::Illinois, "state-association", "2026-10-01")?;
    let failed = failed.record(
        CoachLaneStage::Failed,
        None,
        CoachLaneResolution::unresolved("2026-10-02"),
    )?;

    let pending = lane(UsJurisdiction::Ohio, "state-association", "2026-10-01")?;
    store.replace_many(Table::CoachLanes, &[armed, blocked, failed, pending])?;

    let report = coverage_of(&store, Some(2027))?;
    let contact = report.contact;
    check!(eq; contact.attempted, 3);
    check!(eq; contact.reached, 1);
    check!(eq; contact.blocked, 1);
    check!(eq; contact.failed, 1);
    check!(eq; contact.pending, 1);
    check!(eq; contact.never_attempted, 45);
    check!(eq; contact.successful_empty, 0);
    report.reconcile()?;

    check!(eq; row(&report, "WI")?.state, JurisdictionCoverageState::Complete);
    check!(eq; row(&report, "MN")?.state, JurisdictionCoverageState::Blocked);
    check!(eq; row(&report, "IL")?.state, JurisdictionCoverageState::RetainedGap);
    check!(eq; row(&report, "OH")?.state, JurisdictionCoverageState::Pending);
    check!(eq; row(&report, "KS")?.state, JurisdictionCoverageState::Unresearched);
    Ok(())
}

#[test]
fn terminal_failures_are_explicit_and_not_vacuous() -> TestResult {
    let (_dir, store) = fixture_store()?;
    let mut condition = SourceAccessCondition::new(
        "wiaa",
        "wiaa.org",
        AccessBlockKind::RateLimited,
        429,
        "2026-10-12T00:00:00Z",
        "pressure valve engaged",
    );
    condition.cooldown_until = Some("2099-01-01".to_string());
    store.replace_many(Table::SourceAccess, &[condition])?;

    let report = coverage_of(&store, Some(2027))?;
    check!(
        !report.terminal_failures.is_empty(),
        "a blocking condition is a published failure"
    );
    check!(
        eq;
        report.terminal_failures.get("wiaa:rate_limited"),
        Some(&1)
    );
    Ok(())
}

#[test]
fn an_empty_store_publishes_unresearched_notacles_zero() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let report = coverage_of(&store, Some(2027))?;
    check!(eq; report.source_population.registered, None);
    check!(eq; report.source_population.accepted, None);
    check!(eq; report.contact.attempted, 0);
    check!(eq; report.contact.never_attempted, 49);
    check!(
        eq;
        report.state_counts.get("unresearched"),
        Some(&(UsJurisdiction::CENSUS_SCOPE.len() + 1))
    );
    check!(report
        .jurisdictions
        .iter()
        .all(|row| row.state == JurisdictionCoverageState::Unresearched));
    check!(report
        .jurisdictions
        .iter()
        .all(|row| row.core_share_pct.is_none()));
    report.reconcile()?;
    Ok(())
}
