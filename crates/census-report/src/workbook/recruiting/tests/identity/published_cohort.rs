use super::*;
use census_domain::model::{Confidence, PublishedGraduation, COHORT_UNVERIFIED_FAMILY};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn accepted_alias_published_cohort_survives_collapse_and_publication() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school(&store, UsJurisdiction::Wisconsin, "Published Cohort High")?;
    let mut members = [
        member(
            &school,
            "Published Cohort Runner",
            Gender::Boys,
            "https://wi.milesplit.com/athletes/9001001",
        ),
        member(
            &school,
            "P. Cohort Runner",
            Gender::Boys,
            "https://mn.milesplit.com/athletes/9001001",
        ),
    ];
    members.sort_by(|left, right| left.id.cmp(&right.id));
    let [representative, alias] = &mut members;
    let claim = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "wiaa_results",
            Some("https://wiaa.test/published-cohort/results".to_owned()),
        ),
    };
    let claim_evidence = Evidence::parsed(claim.source.clone(), DAY);
    alias.evidence = vec![claim_evidence.clone()];
    alias.published_graduations.push(claim.clone());
    check!(eq; representative.derived_cohort_confidence(), None);
    check!(eq; alias.derived_cohort_confidence(), Some(Confidence::HIGH));
    check!(representative.observed_grades.is_empty());
    check!(alias.observed_grades.is_empty());
    store.append_many(Table::Athletes, &members)?;
    accept(&store, &members)?;

    let dataset = crate::export::ExportDataset::load(&store)?;
    let [representative, alias] = &members;
    let canonical = dataset
        .identities()
        .canonical_id(alias.id.as_str())
        .to_owned();
    check!(eq; canonical, representative.id.as_str());
    check!(ne; canonical, alias.id.as_str());
    check!(eq; dataset.identities().status(&canonical)?,
    IdentityStatus::Verified);
    let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let [collapsed] = derivation.athletes() else {
        return Err("accepted pair did not publish exactly one athlete".into());
    };
    check!(eq; collapsed.id.as_str(), canonical);
    check!(eq; collapsed.published_graduations, vec![claim.clone()]);
    check!(collapsed.observed_grades.is_empty());
    check!(eq; collapsed.derived_cohort_confidence(),
    Some(Confidence::HIGH));
    check!(collapsed.evidence.contains(&claim_evidence));
    let retained_alias = dataset
        .athletes
        .iter()
        .find(|athlete| athlete.id == alias.id)
        .ok_or("source alias missing from frozen population")?;
    check!(eq; retained_alias.published_graduations, vec![claim]);

    let coverage = crate::report::coverage_report(&dataset, Some(2027))?;
    let wi = coverage
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == "WI")
        .ok_or("Wisconsin coverage missing")?;
    check!(eq; wi.athletes, 1);
    check!(eq; wi.grad_verified, 1);
    check!(eq; wi.grad_unresolved, 0);
    check!(eq; wi.identity_conflicts, 0);
    check!(!coverage.gaps.iter().any(|gap| {
        gap.jurisdiction.code() == "WI"
            && gap.class == crate::report::GapClass::MissingGraduationEvidence
    }));
    let census = crate::report::build_census(&derivation, &store.out_dir());
    check!(eq; census.totals.class_of_2027, 1);
    check!(eq; census.totals.class_of_2027_with_grad_year_evidence, 1);
    check!(eq; coverage.published_totals().athletes, census.totals.athletes);

    let options = crate::workbook::Options {
        school_year: Some(SchoolYear::new(2026).ok_or("invalid fixture school year")?),
        ..crate::workbook::Options::default()
    };
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir());
    let path = crate::workbook::build_from(&dataset, &store, &options, &censuses)?;
    let verified = crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    check!(eq; verified.mapped_athletes, 1);
    let mut book: Xlsx<_> = open_workbook(&path)?;
    let athletes = sheet(&mut book, "Athletes")?;
    check!(eq; athletes.height(), 2);
    let athlete_row = row_of(&athletes, &canonical)?;
    check!(eq; text(
        &athletes,
        athlete_row,
        column_of(&athletes, "Identity Status")?
    ),
    "verified");
    let review = sheet(&mut book, "Review")?;
    let family = column_of(&review, "Family")?;
    let subject = column_of(&review, "Subject ID")?;
    check!(!(1..review.height()).any(|row| {
        text(&review, row, family) == COHORT_UNVERIFIED_FAMILY
            && text(&review, row, subject) == canonical
    }));
    let metrics = sheet(&mut book, "Run Metrics")?;
    let evidence_row = row_of(&metrics, "Class-of-2027 athletes with grade evidence")?;
    check!(eq; text(&metrics, evidence_row, 1), "1");
    check!(eq; text(&metrics, evidence_row, 2), "1");
    check!(eq; text(&metrics, evidence_row, 3), "reconciled");
    Ok(())
}
