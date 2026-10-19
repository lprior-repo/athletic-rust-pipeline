use super::super::super::TestResult;
use census_domain::model::{CanonicalPerformance, GradYear};
use census_domain::public_identity_history::PublicAthleteId;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use std::collections::BTreeMap;
use std::path::Path;

mod fixture;
use fixture::Fixture;

mod identity_history {
    use super::{Fixture, TestResult};
    use census_domain::public_identity_history::PublicAthleteId;
    use std::collections::BTreeMap;

    type IdentityMap = BTreeMap<String, PublicAthleteId>;

    mod source {
        include!("../../../../../../../census-report/src/export/dataset/selected/tests/publication/performance_window/identity_history/source.rs");
    }
    mod correction {
        include!("../../../../../../../census-report/src/export/dataset/selected/tests/publication/performance_window/identity_history/correction.rs");
    }

    pub(super) fn merge_and_split(
        fixture: &mut Fixture,
        alias: census_domain::model::CanonicalAthlete,
    ) -> TestResult<(PublicAthleteId, IdentityMap)> {
        let (case_id, canonical, _merged_ids, alias_proof) = source::merge(fixture, alias)?;
        for references in [&mut fixture.proofs.eligible, &mut fixture.proofs.excluded] {
            references.push(alias_proof.clone());
            references.sort();
            references.dedup();
        }
        let successors = correction::split(fixture, &case_id, &canonical)?;
        Ok((canonical, successors))
    }
}

pub async fn service_split_acceptance() -> TestResult {
    let (mut fixture, alias) = Fixture::synthetic_michigan_reviewed().await?;
    let old_public = fixture.public_id.clone();
    let (_, successors) = identity_history::merge_and_split(&mut fixture, alias)?;
    let successors: BTreeMap<String, String> = successors
        .into_iter()
        .map(|(candidate, public)| (candidate, public.as_str().to_owned()))
        .collect();
    check!(eq; successors.len(), 2);

    let store = &fixture.store;
    let profile = store.snapshot().profile(fixture.selection.profile_id())?;
    let options = super::super::super::selected_options(&profile)?;
    let before_split = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let prior = super::super::super::build(
        store,
        &options,
        "report-source-service-before-split",
        &before_split,
    )?;
    let prior_bytes = std::fs::read(&prior.path)?;
    let prior_manifest_path = Path::new(&prior.path)
        .parent()
        .ok_or("prior service generation")?
        .join("manifest.json");
    let prior_manifest = std::fs::read(&prior_manifest_path)?;

    let old_public_id = PublicAthleteId::parse(&old_public)?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("post-split accepted identity history")?;
    let resolution = accepted.resolve_public(&old_public_id)?;
    let census_domain::public_identity_history::PublicResolution::Ambiguous {
        successors: rows,
        unresolved_bindings: false,
        ..
    } = resolution
    else {
        return Err("post-split historical ID was not ambiguous".into());
    };
    let historical_successors: std::collections::BTreeSet<_> = rows
        .into_iter()
        .map(|row| row.public_id.as_str().to_owned())
        .collect();
    check!(eq;
        historical_successors,
        successors.values().cloned().collect::<std::collections::BTreeSet<_>>()
    );

    let after_split = census_report::export::ExportJobRequest::SelectedPublicationV1 {
        selection: fixture.selection.clone(),
    };
    let current = super::super::super::build(
        store,
        &options,
        "report-source-service-after-split",
        &after_split,
    )?;
    let verified =
        census_report::workbook::publication::verify_published(Path::new(&current.path))?;
    check!(verified.workbook.mapped_athletes > 0);
    let generation = Path::new(&current.path)
        .parent()
        .ok_or("post-split service generation")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    let archive_digest = manifest
        .pointer("/artifacts/frozen-input.json/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or("published frozen archive commitment")?;
    let frozen = ExportDataset::reopen_frozen_with_digest(
        &generation.join("frozen-input.json"),
        archive_digest,
    )?;
    let reopened = census_report::workbook::publication::open_publication(
        Path::new(&current.path),
        &frozen.lineage,
        options.scope,
        GradYear::CO2027,
    )?;
    check!(eq; reopened.generation_digest, verified.generation_digest);
    check!(reopened.workbook.mapped_athletes > 0);

    let original_candidate = fixture.eligible.athlete.to_string();
    let original_public = successors
        .get(&original_candidate)
        .ok_or("eligible split successor")?;
    super::super::super::check_selected_output(
        Path::new(&current.path),
        original_public,
        &original_candidate,
        &fixture.school_id,
        "14-9",
    )?;
    let mut aliases = csv::Reader::from_path(generation.join("identity_aliases.csv"))?;
    let headers = aliases.headers()?.clone();
    let candidate_column = headers
        .iter()
        .position(|value| value == "original_id")
        .ok_or("identity alias candidate column")?;
    let public_column = headers
        .iter()
        .position(|value| value == "public_athlete_id")
        .ok_or("identity alias public ID column")?;
    let alias_rows = aliases.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; alias_rows.len(), successors.len());
    for (candidate, public) in &successors {
        check!(
            alias_rows.iter().any(|row| {
                row.get(candidate_column) == Some(candidate.as_str())
                    && row.get(public_column) == Some(public.as_str())
            }),
            "post-split service alias missing: {candidate} -> {public}"
        );
    }

    let current_bytes = std::fs::read(&current.path)?;
    let retry = super::super::super::build(
        store,
        &options,
        "report-source-service-after-split",
        &after_split,
    )?;
    check!(eq; retry.path, current.path);
    check!(eq;
        census_report::workbook::publication::verify_published(Path::new(&retry.path))?.generation_digest,
        verified.generation_digest
    );
    check!(eq; std::fs::read(&current.path)?, current_bytes);
    check!(eq; std::fs::read(&prior.path)?, prior_bytes);
    check!(eq; std::fs::read(&prior_manifest_path)?, prior_manifest);
    Ok(())
}
