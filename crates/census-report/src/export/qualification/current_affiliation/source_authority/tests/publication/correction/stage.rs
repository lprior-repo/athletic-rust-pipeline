use super::super::retention::files;
use super::*;
use crate::export::public_identity::output::ResolutionAudit;
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::BTreeMap;

type Resolutions = BTreeMap<String, PublicResolution>;

pub(super) struct Stage {
    pub dataset: ExportDataset,
    pub workbook: PathBuf,
    pub public: PublicAthleteId,
    pub(super) resolutions: Resolutions,
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
    resolutions: Resolutions,
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
        let resolutions = oracle::candidate_resolutions(&fixture.store)?;
        let dataset = capture(fixture, objects)?;
        resolution_view(&dataset, &resolutions)?;
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
        resolution_outputs(&workbook, &resolutions, expected)?;
        check!(eq; published::verify_published(&workbook)?.workbook.mapped_athletes, 1);
        check!(eq; published::current_workbook(root)?, workbook);
        let inventory = inventory(&workbook)?;
        Ok(Self {
            dataset,
            workbook,
            public,
            resolutions,
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
            resolutions,
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
            resolutions,
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
        resolution_view(&reopened.dataset, &self.resolutions)?;
        resolution_outputs(&self.workbook, &self.resolutions, expected)?;
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

fn resolution_view(dataset: &ExportDataset, expected: &Resolutions) -> TestResult {
    check!(eq; dataset.athletes.len(), expected.len());
    let view = dataset.verified_public_identities()?;
    for (candidate, resolution) in expected {
        check!(eq; view.candidate_resolution(candidate), Some(resolution));
    }
    Ok(())
}

fn resolution_outputs(workbook: &Path, expected: &Resolutions, selected: &Expected) -> TestResult {
    let directory = workbook.parent().ok_or("identity output generation")?;
    let mut reader = csv::Reader::from_path(directory.join("identity_aliases.csv"))?;
    let headers = reader.headers()?.clone();
    let column = |name| {
        headers
            .iter()
            .position(|value| value == name)
            .ok_or("alias identity column")
    };
    let mut seen = std::collections::BTreeSet::new();
    for record in reader.records() {
        let row = record?;
        check!(seen.len() < expected.len());
        let candidate = row.get(column("original_id")?).ok_or("alias candidate")?;
        check!(seen.insert(candidate.to_owned()));
        let resolution = expected
            .get(candidate)
            .ok_or("unexpected alias candidate")?;
        let (id, status) = match resolution {
            PublicResolution::Canonical(owner) => (&owner.public_id, "canonical"),
            PublicResolution::Redirect { successor, .. } => (&successor.public_id, "redirect"),
            PublicResolution::RevokedRedirect { successor, .. } => {
                (&successor.public_id, "revoked_redirect")
            }
            _ => return Err("controlled candidate lacks unique oracle owner".into()),
        };
        check!(eq; row.get(column("public_athlete_id")?), Some(id.as_str()));
        check!(eq; row.get(column("publication_identity_status")?), Some(status));
        let actual: Value = serde_json::from_str(
            row.get(column("identity_resolution")?)
                .ok_or("alias resolution")?,
        )?;
        check!(eq; actual, serde_json::to_value(ResolutionAudit(resolution))?);
    }
    check!(eq; seen, expected.keys().cloned().collect());
    let mut book: Xlsx<_> = open_workbook(workbook)?;
    let range = book.worksheet_range("Athletes")?;
    check!(eq; range.height(), 2);
    let mut rows = range.rows();
    let headers = rows.next().ok_or("Athletes headers")?;
    let row = rows.next().ok_or("nonempty selected athlete")?;
    let cell = |name| -> TestResult<String> {
        let index = headers
            .iter()
            .position(|value| value.to_string() == name)
            .ok_or("Athletes identity column")?;
        Ok(row.get(index).ok_or("Athletes identity cell")?.to_string())
    };
    check!(eq; cell("Canonical Candidate ID")?, selected.athlete.as_str());
    let resolution = expected
        .get(selected.athlete.as_str())
        .ok_or("selected oracle resolution")?;
    let actual: Value = serde_json::from_str(&cell("Identity Resolution")?)?;
    check!(eq; actual, serde_json::to_value(ResolutionAudit(resolution))?);
    Ok(())
}
