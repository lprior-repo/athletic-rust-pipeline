use super::TestResult;
use crate::export::ExportDataset;
use census_domain::model::{GradYear, ProfileSelectionV1, QueryProfileV1};
use census_store::{Store, Table};
use serde::Deserialize;
use std::path::{Path, PathBuf};

mod embedded;
pub(super) mod embedded_v3;
pub(super) mod files;
mod profile;
mod retention;
mod source;

pub(super) use retention::retain_v3_archive;
pub(super) use source::Expected;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    schema: String,
    producer_commit: String,
    producer_tree: String,
    store_path: PathBuf,
    store_identity: String,
    snapshot_sequence: u64,
    tables_digest: String,
    inventory: serde_json::Value,
    files: Vec<files::File>,
}

pub(super) struct Fixture {
    directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) profile: QueryProfileV1,
    pub(super) expected: Vec<Expected>,
    handoff: Handoff,
    embedded_original: Option<tempfile::TempDir>,
}

impl Fixture {
    pub(super) fn open() -> TestResult<Self> {
        Self::open_for(GradYear::CO2027)
    }

    pub(super) fn open_for(cohort: GradYear) -> TestResult<Self> {
        match std::env::var_os("CENSUS_DOCUMENTARY_HANDOFF_JSON") {
            Some(receipt) => Self::open_from_receipt(PathBuf::from(receipt), cohort),
            None => Self::open_embedded(cohort),
        }
    }

    fn open_from_receipt(receipt: PathBuf, cohort: GradYear) -> TestResult<Self> {
        let metadata = std::fs::symlink_metadata(&receipt)?;
        check!(metadata.file_type().is_file());
        check!(metadata.len() <= 32 * 1024 * 1024);
        let handoff: Handoff = serde_json::from_slice(&std::fs::read(&receipt)?)?;
        check!(eq; handoff.schema, "genuine-documentary-report-handoff/v1");
        check!(files::hex(&handoff.producer_commit, 40));
        check!(files::hex(&handoff.producer_tree, 40));
        check!(eq; handoff.producer_commit, std::env::var("CENSUS_DOCUMENTARY_PRODUCER_COMMIT")?);
        check!(eq; handoff.producer_tree, std::env::var("CENSUS_DOCUMENTARY_PRODUCER_TREE")?);
        Self::assemble(handoff, None, cohort)
    }

    fn open_embedded(cohort: GradYear) -> TestResult<Self> {
        let original = tempfile::tempdir()?;
        let root = original.path().join("original");
        let handoff = embedded::materialize(&root)?;
        Self::assemble(handoff, Some(original), cohort)
    }

    fn assemble(
        handoff: Handoff,
        embedded_original: Option<tempfile::TempDir>,
        cohort: GradYear,
    ) -> TestResult<Self> {
        check!(eq; handoff.inventory.get("attachment_roots").and_then(serde_json::Value::as_u64), Some(2));
        check!(eq; handoff.inventory.get("attachment_physical_rows").and_then(serde_json::Value::as_u64), Some(6));
        check!(eq; handoff.inventory.get("receipt_count").and_then(serde_json::Value::as_u64), Some(6));
        check!(eq; handoff.inventory.get("original_roots").and_then(serde_json::Value::as_array).ok_or("original root inventory")?.len(), 4);
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        files::copy(&handoff.store_path, &root, &handoff.files)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        drop(writer);
        check!(eq; store.store_identity()?.as_str(), handoff.store_identity);
        check!(eq; store.snapshot().sequence(), handoff.snapshot_sequence);
        check!(eq; store.snapshot().tables_digest(&Table::ALL)?, handoff.tables_digest);
        check!(eq; store.receipt_count()?, 6);
        let expected = source::read(&store)?;
        let profile = profile::selected(cohort)?;
        store.register_profile(&profile)?;
        files::verify(&handoff.store_path, &handoff.files)?;
        Ok(Self {
            directory,
            store,
            profile,
            expected,
            handoff,
            embedded_original,
        })
    }

    pub(super) fn capture(&self, directory: &Path) -> TestResult<ExportDataset> {
        let selection = ProfileSelectionV1::new(self.store.store_identity()?, self.profile.id()?);
        Ok(ExportDataset::from_selected_snapshot(
            &self.store,
            &self.store.snapshot(),
            &selection,
            directory,
        )?)
    }

    pub(super) fn provider_school_reference(
        &self,
    ) -> TestResult<census_crawl::participation_evidence::provider_school::ProviderSchoolPacketRefV1>
    {
        let roots = self
            .handoff
            .inventory
            .get("original_roots")
            .and_then(serde_json::Value::as_array)
            .ok_or("handoff original root inventory absent")?;
        let mut matches = roots
            .iter()
            .filter_map(|entry| entry.get("root"))
            .filter(|root| {
                root.get("schema").and_then(serde_json::Value::as_str)
                    == Some("provider_school_packet_v1")
            });
        let root = matches.next().ok_or("provider school packet root absent")?;
        if matches.next().is_some() {
            return Err("ambiguous provider school packet root inventory".into());
        }
        let key = root
            .get("key")
            .and_then(serde_json::Value::as_str)
            .ok_or("provider school root key absent")?;
        Ok(census_crawl::participation_evidence::provider_school::ProviderSchoolPacketRefV1::parse(key)?)
    }

    pub(super) fn require_original_unchanged(&self) -> TestResult {
        files::verify(&self.handoff.store_path, &self.handoff.files)
    }

    pub(super) fn reopen(self) -> TestResult<Self> {
        let (fixture, writer) = self.reopen_with_writer()?;
        drop(writer);
        Ok(fixture)
    }

    pub(super) fn reopen_with_writer(
        self,
    ) -> TestResult<(Self, census_store::PublicIdentityWriterV1)> {
        let Self {
            directory,
            store,
            profile,
            expected,
            handoff,
            embedded_original,
        } = self;
        let reopened = (|| -> TestResult<(Store, census_store::PublicIdentityWriterV1)> {
            files::verify(&handoff.store_path, &handoff.files)?;
            let root = store.root().to_path_buf();
            let identity = store.store_identity()?;
            let sequence = store.snapshot().sequence();
            let tables = store.snapshot().tables_digest(&Table::ALL)?;
            let receipts = store.receipt_count()?;
            drop(store);
            let (reopened, writer) =
                Store::open_public_identity_qualification_v3(&root)?.into_parts();
            check!(eq; reopened.store_identity()?, identity);
            check!(eq; reopened.snapshot().sequence(), sequence);
            check!(eq; reopened.snapshot().tables_digest(&Table::ALL)?, tables);
            check!(eq; reopened.receipt_count()?, receipts);
            files::verify(&handoff.store_path, &handoff.files)?;
            Ok((reopened, writer))
        })();
        match reopened {
            Ok((store, writer)) => Ok((
                Self {
                    directory,
                    store,
                    profile,
                    expected,
                    handoff,
                    embedded_original,
                },
                writer,
            )),
            Err(error) => {
                let _preserved = directory.keep();
                if let Some(original) = embedded_original {
                    let _preserved_original = original.keep();
                }
                Err(error)
            }
        }
    }

    pub(super) fn close(self) -> TestResult {
        let Self {
            directory,
            store,
            handoff,
            ..
        } = self;
        drop(store);
        directory.close()?;
        files::verify(&handoff.store_path, &handoff.files)
    }
}

pub(super) fn copy_publication(workbook: &Path, destination: &Path) -> TestResult<PathBuf> {
    let generation = workbook.parent().ok_or("source publication generation")?;
    let name = generation.file_name().ok_or("source generation name")?;
    let relative = Path::new("generations").join(name);
    let target = destination.join(&relative);
    std::fs::create_dir_all(destination.join("generations"))?;
    let inventory = files::inventory(generation)?;
    files::copy(generation, &target, &inventory)?;
    std::os::unix::fs::symlink(relative, destination.join("current"))?;
    Ok(target.join(workbook.file_name().ok_or("source workbook name")?))
}

pub(super) fn generation_inventory(directory: &Path) -> TestResult<Vec<(String, u64, String)>> {
    Ok(files::inventory(directory)?
        .into_iter()
        .map(|file| (file.path, file.bytes, file.sha256))
        .collect())
}
