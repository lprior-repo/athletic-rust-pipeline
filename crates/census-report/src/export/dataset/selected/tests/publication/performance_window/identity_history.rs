use super::{
    assertions, publication, readback, ExportDataset, Fixture, GradYear, Scope, TestResult,
};
use census_domain::public_identity_history::{PublicAthleteId, PublicResolution};
use std::path::{Path, PathBuf};

mod correction;
mod source;

type IdentityMap = std::collections::BTreeMap<String, PublicAthleteId>;

#[tokio::test]
async fn controlled_eligible_pr_split_preserves_prior_publication_and_store_free_replay(
) -> TestResult {
    let mut fixture = Fixture::synthetic_michigan().await?;
    let profile = super::march_profile()?;
    fixture.select(&profile)?;
    let original_proofs = fixture.proofs.clone();
    let (case_id, original_public, merged_ids, alias_proof) = source::merge(&fixture)?;
    for references in [&mut fixture.proofs.eligible, &mut fixture.proofs.excluded] {
        references.push(alias_proof.clone());
        references.sort();
        references.dedup();
    }
    let prior_objects = super::super::super::temporary()?;
    let current_objects = super::super::super::temporary()?;
    let output = super::super::super::temporary()?;
    let root = output.path().join("publication");
    let prior = fixture.capture(prior_objects.path())?;
    check!(eq; prior.verified_public_identities()?.public_id(fixture.eligible.athlete.as_str()), Some(&original_public));
    check!(eq; prior.verified_public_identities()?.aliases().len(), 1);
    assertions::bounded(&prior, &fixture.eligible, &fixture.stronger, &profile)?;
    let prior_publication = Published::capture(&fixture, &prior, &root, &merged_ids)?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let split_ids = correction::split(&fixture, &case_id, &original_public)?;
    check!(eq; split_ids.values().collect::<std::collections::BTreeSet<_>>().len(), 2);
    check!(split_ids
        .values()
        .all(|id| !merged_ids.values().any(|old| old == id)));
    check!(prior.ensure_current(&fixture.store).is_err());
    check!(fixture.publish(&prior, &root).is_err());
    check!(eq; std::fs::read_link(root.join("current"))?, pointer);
    prior_publication.preserved()?;
    fixture.proofs = original_proofs;
    let current = fixture.capture(current_objects.path())?;
    check!(eq; current.performances, prior.performances);
    check!(eq; current.lineage.source_digest, prior.lineage.source_digest);
    check!(ne; current.lineage.input_generation, prior.lineage.input_generation);
    let identities = current.verified_public_identities()?;
    let replacement = identities
        .public_id(fixture.eligible.athlete.as_str())
        .ok_or("split PR identity")?;
    check!(ne; replacement, &original_public);
    check!(identities.aliases().is_empty());
    check!(eq; identities.public_ids().count(), 4);
    check!(matches!(
        identities.resolve_public(&original_public)?,
        PublicResolution::Ambiguous { .. }
    ));
    assertions::bounded(&current, &fixture.eligible, &fixture.stronger, &profile)?;
    let current_publication = Published::capture(&fixture, &current, &root, &split_ids)?;
    check!(ne; current_publication.expected.public_id, prior_publication.expected.public_id);
    prior_publication.preserved()?;
    let store_path = fixture.store.root().to_path_buf();
    let cache_path = fixture.store.http_cache_dir();
    drop(prior);
    drop(current);
    fixture.remove_sources()?;
    prior_objects.close()?;
    current_objects.close()?;
    check!(!store_path.exists());
    check!(!cache_path.exists());
    prior_publication.replay(&profile, &original_public)?;
    current_publication.replay(&profile, &original_public)?;
    Ok(())
}

struct Published {
    workbook: PathBuf,
    archive_digest: String,
    generation_digest: String,
    lineage: crate::export::DatasetLineage,
    witness: serde_json::Value,
    expected: readback::Expected,
    identities: IdentityMap,
}

impl Published {
    fn capture(
        fixture: &Fixture,
        dataset: &ExportDataset,
        root: &Path,
        identities: &IdentityMap,
    ) -> TestResult<Self> {
        let expected = readback::Expected {
            public_id: identities
                .get(fixture.eligible.athlete.as_str())
                .ok_or("published eligible identity")?
                .as_str()
                .to_owned(),
            school_id: fixture.school_id.clone(),
            eligible: fixture.eligible.clone(),
            excluded: fixture.stronger.clone(),
            result_capture_urls: fixture.result_capture_urls.clone(),
            eligible_evidence_refs: fixture.proofs.eligible.clone(),
            evidence_refs: fixture.proofs.excluded.clone(),
            packets: fixture.proofs.packets.clone(),
            input_generation: dataset.lineage.input_generation.clone(),
        };
        let workbook = fixture.publish(dataset, root)?;
        let directory = workbook.parent().ok_or("publication generation")?;
        readback::artifacts(directory, &workbook, &expected)?;
        aliases(directory, dataset, identities)?;
        coverage(dataset)?;
        let verified = publication::verify_published(&workbook)?;
        check!(eq; verified.workbook.mapped_athletes, 1);
        let generation_digest = verified.generation_digest;
        Ok(Self {
            workbook,
            archive_digest: dataset.archive_digest()?,
            generation_digest,
            lineage: dataset.lineage.clone(),
            witness: super::super::super::witness::capture(dataset)?,
            expected,
            identities: identities.clone(),
        })
    }

    fn preserved(&self) -> TestResult {
        check!(eq; publication::verify_published(&self.workbook)?.generation_digest, self.generation_digest);
        readback::artifacts(
            self.workbook.parent().ok_or("retained generation")?,
            &self.workbook,
            &self.expected,
        )
    }

    fn replay(
        &self,
        profile: &census_domain::model::QueryProfileV1,
        original: &PublicAthleteId,
    ) -> TestResult {
        let directory = self.workbook.parent().ok_or("frozen generation")?;
        let frozen = ExportDataset::reopen_frozen_with_digest(
            &directory.join("frozen-input.json"),
            &self.archive_digest,
        )?;
        let reopened = publication::open_publication(
            &self.workbook,
            &self.lineage,
            Scope::AllSources,
            GradYear::CO2027,
        )?;
        for dataset in [&frozen, &reopened.dataset] {
            check!(eq; super::super::super::witness::capture(dataset)?, self.witness);
            readback::proofs(dataset, &self.expected)?;
            assertions::bounded(
                dataset,
                &self.expected.eligible,
                &self.expected.excluded,
                profile,
            )?;
            aliases(directory, dataset, &self.identities)?;
            coverage(dataset)?;
            let resolution = dataset
                .verified_public_identities()?
                .resolve_public(original)?;
            if self.expected.public_id == original.as_str() {
                check!(matches!(resolution, PublicResolution::Canonical(_)));
            } else {
                let PublicResolution::Ambiguous {
                    successors,
                    unresolved_bindings,
                    ..
                } = resolution
                else {
                    return Err("old public identity must retain split successors".into());
                };
                check!(!unresolved_bindings);
                let actual: IdentityMap = successors
                    .into_iter()
                    .map(|row| (row.representative_candidate.to_string(), row.public_id))
                    .collect();
                check!(eq; actual, self.identities);
            }
        }
        self.preserved()
    }
}

fn aliases(directory: &Path, dataset: &ExportDataset, expected: &IdentityMap) -> TestResult {
    let view = dataset.verified_public_identities()?;
    let mut reader = csv::Reader::from_path(directory.join("identity_aliases.csv"))?;
    let headers = reader.headers()?.clone();
    let candidate = headers
        .iter()
        .position(|value| value == "original_id")
        .ok_or("candidate header")?;
    let public = headers
        .iter()
        .position(|value| value == "public_athlete_id")
        .ok_or("public header")?;
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 2);
    check!(eq; dataset.athletes.len(), expected.len());
    for (athlete, identity) in expected {
        check!(eq; view.public_id(athlete), Some(identity));
        let row = rows
            .iter()
            .find(|row| row.get(candidate) == Some(athlete.as_str()))
            .ok_or("candidate alias row")?;
        check!(eq; row.get(public), Some(identity.as_str()));
    }
    Ok(())
}

fn coverage(dataset: &ExportDataset) -> TestResult {
    let derivation = crate::report::Derivation::of(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[census_domain::UsJurisdiction::Michigan],
    )?;
    let report = crate::report::coverage_report_for_derivation(&derivation)?;
    report.reconcile()?;
    check!(eq; report.raw_observations, 2);
    check!(eq; report.published_totals().athletes, 1);
    let michigan = report
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction.code() == "MI")
        .ok_or("Michigan selected coverage")?;
    check!(eq; michigan.with_pr_support, 1);
    Ok(())
}
