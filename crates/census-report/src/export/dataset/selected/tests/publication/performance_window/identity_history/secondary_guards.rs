use super::{source, ExportDataset, Fixture, TestResult};
use crate::export::dataset::selected::tests::temporary;
use census_domain::model::{SourceIdentity, SourceNamespace};
use census_store::Table;

use crate::export::{ParticipationDiagnostic, ParticipationOutcome};

#[tokio::test]
async fn appended_unreviewed_source_link_after_merge_is_not_promoted() -> TestResult {
    let (mut fixture, alias) = Fixture::synthetic_michigan_reviewed().await?;

    let alias_id = alias.id.clone();

    let (_case_id, original_public, _merged_ids, alias_proof) = source::merge(&fixture, alias)?;
    for references in [&mut fixture.proofs.eligible, &mut fixture.proofs.excluded] {
        references.push(alias_proof.clone());
        references.sort();
        references.dedup();
    }

    let prior_objects = temporary()?;
    let prior = fixture.capture(prior_objects.path())?;
    let archive = temporary()?;
    let archive_path = archive.path().join("before-unreviewed-link.json");
    prior.save_frozen(&archive_path)?;
    let archive_digest = prior.archive_digest()?;
    let prior_identities = prior.verified_public_identities()?;
    check!(
        eq;
        prior_identities.public_id(fixture.eligible.athlete.as_str()),
        Some(&original_public)
    );
    check!(eq; prior_identities.aliases().len(), 1);
    let prior_assessment = prior
        .verified_participation()?
        .assessment(&fixture.eligible.id)
        .ok_or("prior participation")?;
    check!(eq; prior_assessment.outcome(), ParticipationOutcome::Eligible);

    let mut alias_with_new_link = fixture
        .store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|row| row.id == alias_id)
        .ok_or("alias row retained after merge")?;
    alias_with_new_link.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "222000444",
    ));
    fixture
        .store
        .append(Table::Athletes, &alias_with_new_link)?;

    let new_objects = temporary()?;
    let current = fixture.capture(new_objects.path())?;

    let current_assessment = current
        .verified_participation()?
        .assessment(&fixture.eligible.id)
        .ok_or("current participation")?;
    check!(eq; current_assessment.outcome(), ParticipationOutcome::Unresolved);
    check!(current_assessment
        .diagnostics()
        .contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));

    check!(eq; current.verified_public_identities()?.public_id(fixture.eligible.athlete.as_str()), Some(&original_public));

    let candidate = fixture.eligible.athlete.clone();
    let performance = fixture.eligible.id.clone();
    fixture.remove_sources()?;
    drop(prior);
    prior_objects.close()?;
    let reopened = ExportDataset::reopen_frozen_with_digest(&archive_path, &archive_digest)?;
    let prior_identities_after = reopened.verified_public_identities()?;
    check!(
        eq;
        prior_identities_after.public_id(candidate.as_str()),
        Some(&original_public)
    );
    check!(eq; prior_identities_after.aliases().len(), 1);
    check!(eq; reopened.verified_participation()?.assessment(&performance).ok_or("retained prior participation")?.outcome(), ParticipationOutcome::Eligible);
    Ok(())
}
