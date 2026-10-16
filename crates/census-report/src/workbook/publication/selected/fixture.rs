use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use crate::report::{Derivation, Scope};
use crate::workbook::Options;
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, CanonicalSchool, ContractRefV1, Evidence, ExactSeconds,
    Gender, GradYear, Id, Mark, ProfileDate, ProfileId, PublishedGraduation, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_store::{Store, Table};
use std::collections::BTreeMap;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(in crate::workbook) struct Fixture {
    pub dataset: ExportDataset,
    pub options: Options,
    store: Store,
    _directory: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let (school, athlete) = school_and_athlete()?;
        let performances = vec![performance(&athlete)?];
        Self::from_rows(school, vec![athlete], performances)
    }

    pub fn from_rows(
        school: CanonicalSchool,
        athletes: Vec<CanonicalAthlete>,
        performances: Vec<CanonicalPerformance>,
    ) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        store.append(Table::Schools, &school)?;
        store.append_many(Table::Athletes, &athletes)?;
        store.append_many(Table::Performances, &performances)?;
        for athlete in &athletes {
            writer.establish_source_bound(
                &store,
                &format!("selected-fixture-birth-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
        }
        let dataset = ExportDataset::load_public_identity_fixture(&store)?;
        let mut fixture = Self {
            dataset,
            options: Options {
                grad_year: Some(2027),
                out: None,
                limit: Some(0),
                scope: Scope::AllSources,
                jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
                school_year: SchoolYear::new(2026).ok_or("synthetic season")?,
            },
            store,
            _directory: directory,
        };
        fixture.bind('a', "2026-10-01", 'b')?;
        Ok(fixture)
    }

    pub fn bind(&mut self, profile: char, cutoff: &str, rule: char) -> TestResult<()> {
        let profile = ProfileId::parse(&profile.to_string().repeat(64))?;
        let cutoff = ProfileDate::parse(cutoff)?;
        let rule = ContractRefV1::new(1, 1, &rule.to_string().repeat(64))?;
        self.dataset
            .bind_selection_for_test(profile.clone(), cutoff.clone())?;
        self.dataset
            .bind_participation_rule_for_test(rule.clone())?;
        let generation = self.dataset.lineage.input_generation.clone();
        let participation = VerifiedParticipationView::fixture(
            generation.clone(),
            profile.clone(),
            rule,
            BTreeMap::new(),
        );
        let affiliations = VerifiedCurrentAffiliations::fixture(
            generation,
            profile,
            cutoff,
            self.dataset
                .athletes
                .iter()
                .map(|athlete| {
                    (
                        athlete.id.clone(),
                        CurrentAffiliationResolution::Resolved {
                            school: athlete.school.clone(),
                            evidence_refs: vec!["synthetic-selected-affiliation-proof".into()],
                        },
                    )
                })
                .collect(),
        );
        self.dataset
            .bind_verified_views_for_test(participation, affiliations)?;
        self.dataset
            .bind_public_identity_fixture_from_store(&self.store)?;
        Ok(())
    }

    pub fn derivation(&self) -> TestResult<Derivation<'_>> {
        Ok(Derivation::selected(
            &self.dataset,
            self.options.scope,
            self.options.grad_year,
            &self.options.jurisdictions,
            self.dataset.verified_current_affiliations()?,
        )?)
    }
}

pub(in crate::workbook::publication) fn school_and_athlete(
) -> TestResult<(CanonicalSchool, CanonicalAthlete)> {
    let (school, school_id) = CanonicalSchool::new(
        census_domain::UsJurisdiction::Michigan,
        "Synthetic Selected High",
        "synthetic selected high",
        None,
    );
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Synthetic Selected Athlete",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9501"),
    )?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new("synthetic-selected-cohort", None),
    });
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic-selected-fixture",
            Some("https://synthetic-selected.test/athletes/9501".into()),
        ),
        "2026-10-14",
    ));
    Ok((school, athlete))
}

fn performance(athlete: &CanonicalAthlete) -> TestResult<CanonicalPerformance> {
    Ok(CanonicalPerformance {
        id: Id::mint("perf", &["synthetic-selected-missing"]),
        athlete: athlete.id.clone(),
        team: Id::mint("team", &["synthetic-selected"]),
        event: Id::mint("event", &["synthetic-selected"]),
        meet: Id::mint("meet", &["synthetic-selected"]),
        date: "2026-05-01".into(),
        mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
        wind_mps: None,
        adjustment: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: Vec::new(),
        source_key: "synthetic-selected-missing".into(),
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
        combined_completion: None,
    })
}
