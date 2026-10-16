use super::{CanonicalPerformance, ExportDataset, TestResult};
use census_domain::model::{ProfileSelectionV1, QueryProfileV1};
use census_store::Store;
use std::path::{Path, PathBuf};

mod evidence;
mod profile;
mod results;
mod roster;
pub(super) use profile::profile;
pub(super) use results::{raw_body, ResultCase, CASES};

pub(super) struct Fixture {
    directory: tempfile::TempDir,
    pub store: Store,
    pub selection: ProfileSelectionV1,
    pub eligible: CanonicalPerformance,
    pub stronger: CanonicalPerformance,
    pub result_capture_urls: [String; 2],
    pub public_id: String,
    pub school_id: String,
    pub proofs: evidence::ExpectedProofs,
}

impl Fixture {
    pub async fn synthetic_michigan() -> TestResult<Self> {
        let directory = super::super::super::temporary()?;
        let root = directory.path().join("synthetic-mi-store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let ([mut eligible, mut stronger], result_capture_urls) = results::append(&store).await?;
        check!(eq; eligible.athlete, stronger.athlete);
        let (_, school) = roster::append(&store, &eligible.athlete)?;
        let school_id = school.as_str().to_owned();
        let proofs = evidence::append(&store, &mut eligible, &mut stronger)?;
        let profile = profile()?;
        store.register_profile(&profile)?;
        let birth = writer.establish_source_bound(
            &store,
            "synthetic-mi-linked-source-birth",
            &eligible.athlete.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        let public_id = birth.public_id().as_str().to_owned();
        let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
        Ok(Self {
            directory,
            store,
            selection,
            eligible,
            stronger,
            result_capture_urls,
            public_id,
            school_id,
            proofs,
        })
    }

    pub fn select(&mut self, profile: &QueryProfileV1) -> TestResult {
        self.store.register_profile(profile)?;
        self.selection = ProfileSelectionV1::new(self.store.store_identity()?, profile.id()?);
        Ok(())
    }

    pub fn capture(&self, objects: &Path) -> TestResult<ExportDataset> {
        Ok(ExportDataset::from_selected_snapshot(
            &self.store,
            &self.store.snapshot(),
            &self.selection,
            objects,
        )?)
    }

    pub fn publish(&self, dataset: &ExportDataset, root: &Path) -> TestResult<PathBuf> {
        let options = crate::workbook::Options {
            grad_year: Some(2027),
            out: Some(root.to_path_buf()),
            limit: None,
            scope: super::Scope::AllSources,
            jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
            school_year: census_domain::model::SchoolYear::new(2026).ok_or("publication year")?,
        };
        let censuses =
            crate::workbook::Censuses::of(dataset, &self.store.out_dir(), &options.jurisdictions)?;
        Ok(crate::workbook::build_from(
            dataset,
            &self.store,
            &options,
            &censuses,
        )?)
    }

    pub fn remove_sources(self) -> TestResult {
        let Self {
            directory, store, ..
        } = self;
        drop(store);
        directory.close()?;
        Ok(())
    }
}
