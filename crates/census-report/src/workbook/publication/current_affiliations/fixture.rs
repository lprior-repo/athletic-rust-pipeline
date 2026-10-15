use crate::export::{
    AffiliationEvidenceAudit, CurrentAffiliationResolution as Resolution,
    VerifiedCurrentAffiliations, VerifiedParticipationView,
};
use crate::workbook::publication::selected::fixture::{school_and_athlete, Fixture};
use census_domain::model::{
    CanonicalAthlete, Evidence, SourceIdentity, SourceNamespace, SourceRef,
};
use std::collections::BTreeMap;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(in crate::workbook::publication) fn fixture() -> TestResult<Fixture> {
    let (school, original) = school_and_athlete()?;
    let mut athletes = Vec::new();
    let mut rows = BTreeMap::new();
    let mut audits = BTreeMap::new();
    for (index, resolution) in [
        Resolution::Resolved {
            school: original.school.clone(),
            evidence_refs: vec!["synthetic-resolution-only-reference".into()],
        },
        Resolution::Conflicted,
        Resolution::Unknown,
        Resolution::Unknown,
        Resolution::CoopCampusUnresolved,
        Resolution::Unknown,
        Resolution::Unknown,
    ]
    .into_iter()
    .enumerate()
    {
        let mut athlete = CanonicalAthlete::new(
            &original.school,
            format!("Synthetic Audit Athlete {index}"),
            original.grad_year,
            original.gender,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, format!("96{index:02}")),
        )?;
        athlete.published_graduations = original.published_graduations.clone();
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new(
                "synthetic-affiliation-audit-fixture",
                Some(format!(
                    "https://synthetic-selected.test/athletes/96{index:02}"
                )),
            ),
            "2026-10-14",
        ));
        if index < 6 {
            rows.insert(athlete.id.clone(), resolution);
            audits.insert(
                athlete.id.clone(),
                match index {
                    3 => AffiliationEvidenceAudit::fixture(
                        Vec::new(),
                        vec!["outside_capture_day".into()],
                    ),
                    5 => AffiliationEvidenceAudit::fixture(
                        Vec::new(),
                        vec!["no_retained_row".into()],
                    ),
                    _ => AffiliationEvidenceAudit::fixture(
                        vec![
                            format!("synthetic-audit-reference-{index}"),
                            "synthetic-shared-reference".into(),
                        ],
                        vec!["outside_selected_sport".into()],
                    ),
                },
            );
        }
        athletes.push(athlete);
    }
    let mut fixture = Fixture::from_rows(school, athletes, Vec::new())?;
    let dataset = &mut fixture.dataset;
    let generation = dataset.lineage.input_generation.clone();
    let profile = dataset
        .selected_profile_id()
        .ok_or("selected profile")?
        .clone();
    let cutoff = dataset.selected_as_of().ok_or("selected cutoff")?.clone();
    let rule = dataset
        .selected_participation_rule()
        .ok_or("selected rule")?
        .clone();
    dataset.bind_verified_views_for_test(
        VerifiedParticipationView::fixture(
            generation.clone(),
            profile.clone(),
            rule,
            BTreeMap::new(),
        ),
        VerifiedCurrentAffiliations::fixture_with_audit(generation, profile, cutoff, rows, audits),
    )?;
    Ok(fixture)
}
