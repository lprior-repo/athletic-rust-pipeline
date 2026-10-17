use super::super::retention::files;
use super::*;

pub(super) struct Stage {
    pub dataset: ExportDataset,
    pub workbook: PathBuf,
    pub public: PublicAthleteId,
    inventory: Value,
    observations: Value,
    rows: Value,
    identities: Value,
}

pub(super) struct Retained {
    workbook: PathBuf,
    lineage: crate::export::DatasetLineage,
    inventory: Value,
    observations: Value,
    rows: Value,
    identities: Value,
    digest: String,
}

impl Stage {
    pub(super) fn capture(
        fixture: &fixture::Fixture,
        expected: &Expected,
        identities: &[PublicAthleteId],
        objects: &Path,
        root: &Path,
    ) -> TestResult<Self> {
        check!(eq; identities.len(), 2);
        let dataset = capture(fixture, objects)?;
        let view = dataset.verified_public_identities()?;
        let public = view
            .public_id(expected.athlete.as_str())
            .ok_or("qualified controlled public identity")?
            .clone();
        check!(eq; Some(&public), identities.first());
        check!(eq; dataset.athletes.len(), 2);
        let other = dataset
            .athletes
            .iter()
            .find(|row| row.id != expected.athlete)
            .ok_or("synthetic second candidate")?;
        check!(eq; view.public_id(other.id.as_str()), identities.last());
        let affiliations = dataset.verified_current_affiliations()?;
        if identities.first() != identities.last() {
            check!(eq; affiliations.resolution(&other.id).status(), "current_affiliation_unknown");
        }
        let identities = identity_inventory(&dataset)?;
        let observations = witness(&dataset, expected)?;
        let workbook = publish(fixture, &dataset, root)?;
        let rows = artifacts::readback(&workbook, &dataset, expected)?;
        check!(eq; published::verify_published(&workbook)?.workbook.mapped_athletes, 1);
        check!(eq; published::current_workbook(root)?, workbook);
        let inventory = inventory(&workbook)?;
        Ok(Self {
            dataset,
            workbook,
            public,
            inventory,
            observations,
            rows,
            identities,
        })
    }

    pub(super) fn unchanged(&self) -> TestResult {
        check!(eq; inventory(&self.workbook)?, self.inventory);
        Ok(())
    }

    pub(super) fn freeze(self) -> TestResult<Retained> {
        self.unchanged()?;
        let digest = self.dataset.archive_digest()?;
        let Self {
            dataset,
            workbook,
            inventory,
            observations,
            rows,
            identities,
            ..
        } = self;
        let lineage = dataset.lineage.clone();
        drop(dataset);
        Ok(Retained {
            workbook,
            lineage,
            inventory,
            observations,
            rows,
            identities,
            digest,
        })
    }
}

impl Retained {
    pub(super) fn replay(&self, expected: &Expected) -> TestResult {
        let reopened = published::open_publication(
            &self.workbook,
            &self.lineage,
            Scope::AllSources,
            GradYear::CO2027,
        )?;
        check!(eq; reopened.dataset.archive_digest()?, self.digest);
        check!(eq; witness(&reopened.dataset, expected)?, self.observations);
        check!(eq; identity_inventory(&reopened.dataset)?, self.identities);
        check!(eq; artifacts::readback(&self.workbook, &reopened.dataset, expected)?, self.rows);
        check!(eq; reopened.workbook.mapped_athletes, 1);
        check!(eq; inventory(&self.workbook)?, self.inventory);
        Ok(())
    }
}

pub(super) fn stale(dataset: &ExportDataset, store: &Store) -> TestResult {
    let before = (store.snapshot().sequence(), store.receipt_count()?);
    check!(
        matches!(dataset.ensure_current(store), Err(crate::report::ReportError::Invariant { detail }) if detail == "selected public identity authority changed")
    );
    check!(
        matches!(dataset.ensure_snapshot(&store.snapshot()), Err(crate::report::ReportError::Invariant { detail }) if detail == "selected public identity authority changed")
    );
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?), before);
    Ok(())
}

fn inventory(workbook: &Path) -> TestResult<Value> {
    Ok(serde_json::to_value(files::inventory(
        workbook.parent().ok_or("publication generation")?,
    )?)?)
}

fn identity_inventory(dataset: &ExportDataset) -> TestResult<Value> {
    let identities = dataset.verified_public_identities()?;
    let affiliations = dataset.verified_current_affiliations()?;
    Ok(serde_json::to_value(
        dataset
            .athletes
            .iter()
            .map(|row| {
                (
                    &row.id,
                    identities.public_id(row.id.as_str()),
                    affiliations.resolution(&row.id).status(),
                )
            })
            .collect::<Vec<_>>(),
    )?)
}
