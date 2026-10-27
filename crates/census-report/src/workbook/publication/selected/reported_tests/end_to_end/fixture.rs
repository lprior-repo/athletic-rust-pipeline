use super::support::{athlete, event_context, include_profile, performance};
use super::TestResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, CanonicalSchool, Evidence, ProfileSelectionV1,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) struct IncludedFixture {
    pub(super) directory: tempfile::TempDir,
    pub(super) _objects: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) dataset: crate::export::ExportDataset,
    pub(super) options: crate::workbook::Options,
    pub(super) first: CanonicalAthlete,
    pub(super) second: CanonicalAthlete,
    pub(super) first_performance: CanonicalPerformance,
    pub(super) faster_performance: CanonicalPerformance,
    pub(super) second_performance: CanonicalPerformance,
}

impl IncludedFixture {
    pub(super) fn new() -> TestResult<Self> {
        Self::with_evidence(None)
    }

    pub(super) fn with_aggregate_evidence(
        athlete_note: &str,
        performance_note: &str,
    ) -> TestResult<Self> {
        Self::with_evidence(Some((athlete_note, performance_note)))
    }

    fn with_evidence(notes: Option<(&str, &str)>) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("selected-store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let (mut school, school_id) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            "Synthetic Michigan High",
            "synthetic michigan high",
            None,
        );
        school.source_identities.push(SourceIdentity::new(
            SourceNamespace::MilesplitSchool,
            "95010",
        ));
        school.evidence.push(Evidence::parsed(
            SourceRef::new(
                "milesplit",
                Some("https://mi.milesplit.com/teams/95010".into()),
            ),
            "2026-10-14",
        ));
        let mut first = athlete(&school_id, "First Runner", "9501")?;
        let second = athlete(&school_id, "Second Runner", "9502")?;
        let (team, first_meet, first_event) =
            event_context(&school_id, first.gender, "2026-04-10", "First 400m Meet")?;
        let (_, second_meet, second_event) =
            event_context(&school_id, first.gender, "2026-05-11", "Second 400m Meet")?;
        let (_, third_meet, third_event) =
            event_context(&school_id, first.gender, "2026-06-12", "Third 400m Meet")?;
        let first_performance = performance(
            &first,
            &team,
            &first_meet,
            &first_event,
            "reported-50",
            "50.00",
        )?;
        let mut faster_performance = performance(
            &first,
            &team,
            &second_meet,
            &second_event,
            "reported-49",
            "49.00",
        )?;
        let second_performance = performance(
            &second,
            &team,
            &third_meet,
            &third_event,
            "reported-55",
            "55.00",
        )?;
        if let Some((athlete_note, performance_note)) = notes {
            first.evidence.first_mut().ok_or("athlete evidence")?.note =
                Some(athlete_note.to_owned());
            let mut additional = first.evidence.first().ok_or("athlete evidence")?.clone();
            additional.note = Some(format!("{athlete_note}-additional"));
            first.evidence.push(additional);
            faster_performance
                .evidence
                .first_mut()
                .ok_or("performance evidence")?
                .note = Some(performance_note.to_owned());
        }
        store.append(Table::Schools, &school)?;
        store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
        store.append(Table::Teams, &team)?;
        store.append_many(
            Table::Meets,
            &[first_meet.clone(), second_meet.clone(), third_meet.clone()],
        )?;
        store.append_many(Table::Events, &[first_event, second_event, third_event])?;
        store.append_many(
            Table::Performances,
            &[
                first_performance.clone(),
                faster_performance.clone(),
                second_performance.clone(),
            ],
        )?;
        for athlete in [&first, &second] {
            writer.establish_source_bound(
                &store,
                &format!("reported-only-birth-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
        }
        let profile = include_profile()?;
        store.register_profile(&profile)?;
        let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
        let objects = tempfile::tempdir()?;
        let dataset = crate::export::ExportDataset::for_job_request(
            &store,
            "reported-include-publication-v1",
            &crate::export::ExportJobRequest::SelectedPublicationV1 { selection },
        )?;
        let options = crate::workbook::Options {
            address_corpus: None,
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("school year")?,
            jurisdictions: vec![UsJurisdiction::Michigan],
        };
        Ok(Self {
            directory,
            _objects: objects,
            store,
            dataset,
            options,
            first,
            second,
            first_performance,
            faster_performance,
            second_performance,
        })
    }
}
