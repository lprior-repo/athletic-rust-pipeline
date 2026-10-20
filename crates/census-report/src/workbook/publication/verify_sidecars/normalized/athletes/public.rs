use super::super::rows::Row;
use crate::export::public_identity::output::ResolutionAudit;
use crate::export::public_identity::population::IdentityPopulation;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::verify::public_identity::Expected;
use census_domain::public_identity_history::PublicResolution;

pub(in crate::workbook::publication::verify_sidecars::normalized) fn public_identity_row(
    row: &Row<'_>,
    identities: &IdentityPopulation<'_>,
    candidate: &str,
) -> ReportResult<()> {
    let expected = Expected::of(identities, candidate)?;
    row.text("athlete_id", expected.athlete_id)?;
    if let Some([candidate, resolution]) = expected.audit {
        row.text("canonical_candidate_id", candidate)?;
        row.text("identity_resolution", resolution)?;
    }
    Ok(())
}

pub(super) fn alias_headers(dataset: &ExportDataset) -> Vec<&'static str> {
    let additional = dataset.selected_profile_id().map(|_| {
        [
            "public_athlete_id",
            "publication_identity_status",
            "identity_resolution",
        ]
    });
    super::schema::ALIASES
        .iter()
        .copied()
        .chain(additional.into_iter().flatten())
        .collect()
}

pub(super) fn alias_public_identity(
    row: &Row<'_>,
    identities: &IdentityPopulation<'_>,
) -> ReportResult<()> {
    let Some(view) = identities.selected_view() else {
        return Ok(());
    };
    let candidate = row.get("original_id")?;
    let group = view
        .group(candidate)
        .ok_or_else(|| super::rows::missing("public identity candidate", candidate))?;
    let resolution = view
        .candidate_resolution(candidate)
        .ok_or_else(|| super::rows::missing("public identity resolution", candidate))?;
    let status = match resolution {
        PublicResolution::Unknown => "unknown",
        PublicResolution::Unresolved => "unresolved",
        PublicResolution::Ambiguous { .. } => "ambiguous",
        _ if group.public_id().is_none() => "candidate_group_unresolved",
        PublicResolution::Canonical(_) => "canonical",
        PublicResolution::Redirect { .. } => "redirect",
        PublicResolution::RevokedRedirect { .. } => "revoked_redirect",
    };
    row.text(
        "public_athlete_id",
        group.public_id().map_or("", |id| id.as_str()),
    )?;
    row.text("publication_identity_status", status)?;
    row.json("identity_resolution", &ResolutionAudit(resolution))
}
