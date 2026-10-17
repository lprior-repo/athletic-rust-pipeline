use super::*;
use crate::export::DatasetLineage;

pub(super) struct Prior {
    pub workbook: PathBuf,
    archive: PathBuf,
    digest: String,
    lineage: DatasetLineage,
    generation_digest: String,
    inventory: Vec<(String, u64, String)>,
    pointer: PathBuf,
    observed: Value,
    expected: Vec<Expected>,
}

impl Prior {
    pub(super) fn publish(
        fixture: &Fixture,
        objects: &Path,
        archives: &Path,
        root: &Path,
    ) -> TestResult<Self> {
        let dataset = fixture.capture(objects)?;
        let observed = inspect(&dataset, &fixture.expected)?;
        let archive = archives.join("genuine-documentary.json");
        dataset.save_frozen(&archive)?;
        let digest = dataset.archive_digest()?;
        let lineage = dataset.lineage.clone();
        let workbook = super::super::publish(fixture, &dataset, root)?;
        drop(dataset);
        let generation = workbook.parent().ok_or("prior generation")?;
        let inventory = fixture::generation_inventory(generation)?;
        let generation_digest = publication::verify_published(&workbook)?.generation_digest;
        let pointer = std::fs::read_link(root.join("current"))?;
        Ok(Self {
            workbook,
            archive,
            digest,
            lineage,
            generation_digest,
            inventory,
            pointer,
            observed,
            expected: fixture.expected.clone(),
        })
    }

    pub(super) fn preserved(&self, root: &Path) -> TestResult {
        check!(eq; std::fs::read_link(root.join("current"))?, self.pointer);
        self.generation_preserved()
    }

    pub(super) fn generation_preserved(&self) -> TestResult {
        check!(eq; fixture::generation_inventory(self.workbook.parent().ok_or("prior generation")?)?, self.inventory);
        check!(eq; publication::verify_published(&self.workbook)?.generation_digest, self.generation_digest);
        Ok(())
    }

    pub(super) fn replay(&self) -> TestResult {
        let dataset = ExportDataset::reopen_frozen_with_digest(&self.archive, &self.digest)?;
        check!(eq; dataset.lineage, self.lineage);
        check!(eq; inspect(&dataset, &self.expected)?, self.observed);
        drop(dataset);
        let opened = publication::open_publication(
            &self.workbook,
            &self.lineage,
            Scope::AllSources,
            GradYear::CO2027,
        )?;
        check!(eq; inspect(&opened.dataset, &self.expected)?, self.observed);
        csv(
            self.workbook.parent().ok_or("prior generation")?,
            &opened.dataset,
            &self.expected,
        )?;
        drop(opened);
        self.generation_preserved()
    }
}
