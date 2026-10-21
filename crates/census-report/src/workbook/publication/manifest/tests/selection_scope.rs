use super::super::{Manifest, Selection};
use super::TestResult;
use crate::export::{
    decoded_generations, generation_counter_window, resident_generations,
    selected_test_fixture::Fixture,
};
use crate::report::{ReportError, Scope};
use crate::workbook::{self, publication, Censuses, Options};
use census_domain::model::{GradYear, ProfileSelectionV1, QueryProfileV1, SchoolYear};
use census_domain::UsJurisdiction;
use std::path::PathBuf;

struct Published {
    fixture: Fixture,
    workbook: PathBuf,
    manifest: Manifest,
    original: Vec<u8>,
}

impl Published {
    async fn new() -> TestResult<Self> {
        let mut fixture = Fixture::new().await?;
        let mut input = fixture
            .store
            .snapshot()
            .selected_profile(&fixture.selection)?
            .input()
            .clone();
        input.jurisdictions = vec![
            UsJurisdiction::Alabama,
            UsJurisdiction::WestVirginia,
            UsJurisdiction::Wisconsin,
        ];
        let profile = QueryProfileV1::new(input)?;
        fixture.store.register_profile(&profile)?;
        fixture.selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
        let objects = tempfile::tempdir()?;
        let dataset = fixture.capture(objects.path())?;
        let options = Options {
            scope: Scope::AllSources,
            grad_year: Some(2027),
            limit: None,
            school_year: SchoolYear::new(2026).ok_or("contact year")?,
            jurisdictions: profile.input().jurisdictions.clone(),
            out: None,
        };
        let censuses = Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
        let workbook = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
        let original = std::fs::read(workbook.parent().ok_or("generation")?.join("manifest.json"))?;
        let manifest = serde_json::from_slice(&original)?;
        drop(dataset);
        objects.close()?;
        Ok(Self {
            fixture,
            workbook,
            manifest,
            original,
        })
    }

    fn manifest_path(&self) -> TestResult<PathBuf> {
        Ok(self
            .workbook
            .parent()
            .ok_or("generation")?
            .join("manifest.json"))
    }

    fn seal(
        &self,
        jurisdictions: &[UsJurisdiction],
    ) -> crate::report::ReportResult<publication::OpenedPublication> {
        publication::open_publication_for_seal(
            &self.workbook,
            &self.fixture.store,
            Scope::AllSources,
            GradYear::CO2027,
            jurisdictions,
        )
    }

    fn substitute(&mut self, jurisdictions: &[UsJurisdiction]) -> TestResult {
        self.manifest.selection.jurisdictions = jurisdictions
            .iter()
            .map(|jurisdiction| jurisdiction.code().to_string())
            .collect();
        self.manifest.generation_digest = self.manifest.digest()?;
        std::fs::write(self.manifest_path()?, serde_json::to_vec(&self.manifest)?)?;
        let checked = super::super::verify_directory(self.workbook.parent().ok_or("generation")?)?;
        check!(eq; checked.generation_digest, self.manifest.generation_digest);
        Ok(())
    }
}

#[tokio::test]
async fn selected_manifest_rejects_rehashed_jurisdiction_substitution() -> TestResult {
    let mut published = Published::new().await?;
    let _window = generation_counter_window()?;
    let workbook = std::fs::read(&published.workbook)?;
    let pointer = std::fs::read_link(
        published
            .fixture
            .store
            .out_dir()
            .join("publication/current"),
    )?;
    for jurisdictions in [
        vec![UsJurisdiction::Alabama],
        vec![
            UsJurisdiction::Alabama,
            UsJurisdiction::Michigan,
            UsJurisdiction::Wisconsin,
            UsJurisdiction::WestVirginia,
        ],
        Vec::new(),
    ] {
        published.substitute(&jurisdictions)?;
        refusal_uses_one_generation(|| publication::verify_published(&published.workbook))?;
        refusal_uses_one_generation(|| published.seal(&jurisdictions))?;
        check!(eq; std::fs::read(&published.workbook)?, workbook);
        check!(eq; std::fs::read_link(published.fixture.store.out_dir().join("publication/current"))?, pointer);
    }
    std::fs::write(published.manifest_path()?, &published.original)?;
    publication::verify_published(&published.workbook)?;
    Ok(())
}

fn refusal_uses_one_generation<T>(
    open: impl FnOnce() -> crate::report::ReportResult<T>,
) -> TestResult {
    let decoded = decoded_generations();
    let resident = resident_generations();
    let error = open()
        .err()
        .ok_or("rehashed jurisdiction substitution accepted")?;
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == "manifest selection differs from the frozen selected profile"));
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("decode counter overflow")?);
    check!(eq; resident_generations(), resident);
    Ok(())
}

#[tokio::test]
async fn selected_manifest_restores_profile_order_with_one_resident_generation() -> TestResult {
    let published = Published::new().await?;
    let jurisdictions = vec![
        UsJurisdiction::Alabama,
        UsJurisdiction::WestVirginia,
        UsJurisdiction::Wisconsin,
    ];
    check!(eq; published.manifest.selection.jurisdictions, vec!["AL", "WI", "WV"]);
    let _window = generation_counter_window()?;
    let decoded = decoded_generations();
    let resident = resident_generations();
    let opened = published.seal(&jurisdictions)?;
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("decode counter overflow")?);
    check!(eq; resident_generations(), resident.checked_add(1).ok_or("resident counter overflow")?);
    let options = workbook::options_for_selected_dataset(&opened.dataset, None)?;
    check!(eq; options.jurisdictions, jurisdictions);
    check!(eq; Selection::of(&options), published.manifest.selection);
    drop(opened);
    check!(eq; resident_generations(), resident);
    Ok(())
}
