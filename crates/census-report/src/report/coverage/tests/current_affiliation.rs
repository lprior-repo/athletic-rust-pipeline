use super::*;
use crate::export::{CurrentAffiliationResolution, VerifiedCurrentAffiliations};
use crate::report::Derivation;
use census_domain::model::{
    CoachLaneObligation, CoachLaneResolution, CoachLaneStage, CoverageRow, CoverageScope,
    PendingCoachLane, ProfileDate, ProfileId,
};
use std::collections::HashSet;

fn bind(dataset: &mut ExportDataset, store: &Store) -> TestResult {
    dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    dataset.bind_participation_rule_for_test(
        census_domain::participation_candidates::participation_policy_v2()?,
    )?;
    dataset.bind_public_identity_fixture_from_store(store)?;
    Ok(())
}

fn selected_fixture_store() -> TestResult<(TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    Store::create_public_identity_qualification_v3(directory.path())?;
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path())?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    populate_fixture(&store, |id| match id {
        "wi-mirror" => SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9702"),
        _ => fixture_source(id),
    })?;
    for mut athlete in store.snapshot().athletes()?.into_iter().filter(|athlete| {
        matches!(
            athlete.canonical_name.as_str(),
            "Julian Aguilera" | "Rae Lindgren"
        )
    }) {
        let source = athlete.source.as_ref().ok_or("source identity")?;
        athlete
            .evidence
            .first_mut()
            .ok_or("source evidence")?
            .source
            .url = Some(format!(
            "https://synthetic-coverage.test/athletes/{}",
            source.id
        ));
        store.append(Table::Athletes, &athlete)?;
        writer.establish_source_bound(
            &store,
            &format!("coverage-source-birth-{}", athlete.id),
            &athlete.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
    }
    Ok((directory, store))
}

fn bind_views(
    dataset: &mut ExportDataset,
    rows: BTreeMap<census_domain::model::AthleteId, CurrentAffiliationResolution>,
) -> TestResult {
    let view = affiliations(dataset, rows)?;
    let participation = crate::export::VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        ProfileId::parse(&"a".repeat(64))?,
        census_domain::participation_candidates::participation_policy_v2()?,
        BTreeMap::new(),
    );
    dataset.bind_verified_views_for_test(participation, view)?;
    Ok(())
}

fn affiliations(
    dataset: &ExportDataset,
    rows: BTreeMap<census_domain::model::AthleteId, CurrentAffiliationResolution>,
) -> TestResult<VerifiedCurrentAffiliations> {
    Ok(VerifiedCurrentAffiliations::fixture(
        dataset.lineage.input_generation.clone(),
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
        rows,
    ))
}

fn report_shell(derivation: &Derivation<'_>, state: UsJurisdiction) -> CoverageReport {
    CoverageReport {
        grad_year: None,
        event_catalog: Default::default(),
        jurisdictions: vec![JurisdictionCoverage {
            jurisdiction: state.into(),
            ..Default::default()
        }],
        gaps: Vec::new(),
        read: CoverageTotals {
            schools: derivation.schools().len(),
            athletes: derivation.athletes().len(),
            coaches: derivation.coaches().len(),
            meets: derivation.meets().len(),
            performances: derivation.performances().len(),
        },
        off_cohort_athletes: 0,
        notes: Vec::new(),
        source_population: Default::default(),
        state_counts: BTreeMap::new(),
        contact: Default::default(),
        terminal_failures: BTreeMap::new(),
        raw_observations: derivation.dataset().athletes.len(),
        scope_filtered: derivation.scoped_athletes(),
        accepted_cohort: 0,
    }
}

#[test]
fn selected_retained_diagnostics_keep_outside_contact_and_excluded_orphan_context() -> TestResult {
    let (_directory, store) = selected_fixture_store()?;
    let mut dataset = ExportDataset::load_public_identity_fixture(&store)?;
    for (revision, stage) in [(1, CoachLaneStage::Blocked), (2, CoachLaneStage::Failed)] {
        let pending = CoachLaneObligation::pending(PendingCoachLane {
            jurisdiction: UsJurisdiction::Wisconsin,
            season: SchoolYear::new(2026).ok_or("season")?,
            revision,
            arm: format!("fixture-{stage:?}"),
            arm_basis: "state association directory".into(),
            observed_on: "2026-09-01".into(),
        })?;
        dataset.coverage_evidence.coach_lanes.push(pending.record(
            stage,
            None,
            CoachLaneResolution::unresolved("2026-09-02"),
        )?);
    }
    dataset.coverage_evidence.rows.push(
        CoverageRow::new(CoverageScope::Jurisdiction, "WI")
            .with("registered", 7)
            .with("attempted", 2),
    );
    bind(&mut dataset, &store)?;
    bind_views(&mut dataset, BTreeMap::new())?;
    let derivation = Derivation::of(
        &dataset,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Michigan],
    )?;
    let mut report = report_shell(&derivation, UsJurisdiction::Michigan);
    retained::append(&mut report, &derivation)?;
    let outside = report
        .notes
        .iter()
        .find(|note| {
            note.starts_with("retained historical/raw outside requested jurisdictions WI:")
        })
        .ok_or("missing retained Wisconsin contact/source lifecycle")?;
    check!(outside.contains("registered=7,applicable=unmeasured,attempted=2"));
    check!(outside.contains("contact_attempted=2"));
    check!(outside.contains("contact_failed=1 contact_blocked=1"));
    for (jurisdiction, class) in [
        ("WI", "missing_event_context"),
        ("WI", "unmapped_event"),
        (JurisdictionBucket::UNPLACED_CODE, "missing_event_context"),
    ] {
        check!(report.notes.iter().any(|note| {
            note.starts_with("retained historical/raw gap ")
                && note.contains(&format!(
                    "jurisdiction={jurisdiction} class={class} unit=performances count=1"
                ))
        }));
    }
    check!(!report.notes.iter().any(|note| {
        note.starts_with("retained historical/raw gap ")
            && note.contains("class=missing_pr_support")
    }));
    check!(eq; report.read.athletes, 0);
    check!(eq; report.read.performances, 0);
    Ok(())
}

#[test]
fn alias_transfer_uses_current_canonical_resolution_for_population_pr_and_outside_counts(
) -> TestResult {
    let (_directory, historical_store) = fixture_store()?;
    let historical_dataset = ExportDataset::load(&historical_store)?;
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    let merged = crate::export::public_identity::tests::fixture::merge(&fixture)?;
    let mut dataset = ExportDataset::load_public_identity_fixture(&fixture.store)?;
    let [original, alias, _] = &fixture.athletes;
    let mut historical = historical_dataset
        .performances
        .iter()
        .find(|row| row.source_key == "wi-1")
        .cloned()
        .ok_or("historical performance")?;
    historical.athlete = original.id.clone();
    historical.id = CanonicalPerformance::mint(
        &original.id,
        &historical.meet,
        &historical.event,
        &historical.date,
        "original-result",
    );
    let mut alias_result = historical.clone();
    alias_result.athlete = alias.id.clone();
    alias_result.id = CanonicalPerformance::mint(
        &alias.id,
        &historical.meet,
        &historical.event,
        &historical.date,
        "alias-result",
    );
    dataset.performances = vec![historical, alias_result];
    dataset
        .athletes
        .retain(|row| row.id == original.id || row.id == alias.id);
    dataset.meets = historical_dataset.meets.clone();
    dataset.events = historical_dataset.events.clone();
    let (mut previous, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Previous High",
        "previous high",
        None,
    );
    previous.id = original.school.clone();
    dataset.schools = [(previous.id.clone(), previous)].into_iter().collect();
    dataset.coaches.clear();
    dataset.coach_observations.clear();
    let (school, current) = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        "Current High",
        "current high",
        None,
    );
    dataset.schools.insert(current.clone(), school);
    bind(&mut dataset, &fixture.store)?;
    check!(eq; dataset.verified_public_identities()?.head(), merged.head());
    bind_views(
        &mut dataset,
        [(
            original.id.clone(),
            CurrentAffiliationResolution::Resolved {
                school: current.clone(),
                evidence_refs: vec!["dated-transfer".into()],
            },
        )]
        .into_iter()
        .collect(),
    )?;
    let unchanged = serde_json::to_vec(&dataset)?;
    for (state, athletes, performances, pr, outside) in [
        (
            UsJurisdiction::Michigan,
            1,
            2,
            1,
            "schools=1 athletes=0 coaches=0 meets=1 performances=0",
        ),
        (
            UsJurisdiction::Wisconsin,
            0,
            0,
            0,
            "schools=1 athletes=1 coaches=0 meets=0 performances=2",
        ),
    ] {
        let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[state])?;
        let mut report = report_shell(&derivation, state);
        let supported = HashSet::from([original.id.as_str()]);
        pr_support::tally_pr_support_for_derivation(&mut report, &derivation, &supported);
        retained::append(&mut report, &derivation)?;
        check!(eq; report.read.athletes, athletes);
        check!(eq; report.read.performances, performances);
        check!(eq; report.jurisdictions.first().ok_or("coverage row")?.with_pr_support, pr);
        check!(report.notes.iter().any(|note| note.contains(&format!(
            "known outside requested jurisdictions [{outside}]"
        ))));
        if let Some(athlete) = derivation.athletes().first() {
            check!(eq; &athlete.id, &original.id);
            check!(eq; &athlete.school, &current);
            check!(derivation.affiliation_exclusions().is_empty());
        }
    }
    check!(eq; serde_json::to_vec(&dataset)?, unchanged);
    check!(eq; &dataset.performances.get(1).ok_or("alias result")?.athlete, &alias.id);
    Ok(())
}

#[test]
fn legacy_alias_map_cannot_authorize_selected_coverage() -> TestResult {
    let (_directory, store) = fixture_store()?;
    let mut dataset = ExportDataset::load(&store)?;
    let canonical = dataset
        .athletes
        .first()
        .ok_or("canonical candidate")?
        .id
        .to_string();
    let alias = dataset
        .athletes
        .get(1)
        .ok_or("alias candidate")?
        .id
        .to_string();
    dataset.canonical_aliases.insert(alias, canonical);
    dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    check!(
        matches!(Derivation::of(&dataset, Scope::AllSources, None, &[]),
        Err(ReportError::Invariant { detail })
        if detail == "selected publication requires retained public-identity verification")
    );
    Ok(())
}
