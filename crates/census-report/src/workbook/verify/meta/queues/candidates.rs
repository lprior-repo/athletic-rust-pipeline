use super::super::Expect;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::public_identity_history::PublicResolution;

pub(super) fn expected(dataset: &ExportDataset) -> ReportResult<Vec<Vec<Expect>>> {
    let view = dataset.verified_public_identities()?;
    let mut rows = Vec::new();
    for athlete in &dataset.athletes {
        if view
            .group(athlete.id.as_str())
            .is_some_and(|group| group.public_id().is_some())
        {
            continue;
        }
        let state = match view.candidate_resolution(athlete.id.as_str()) {
            Some(PublicResolution::Unknown) => "unknown",
            Some(PublicResolution::Unresolved) => "unresolved",
            Some(PublicResolution::Ambiguous { .. }) => "ambiguous",
            Some(
                PublicResolution::Canonical(_)
                | PublicResolution::Redirect { .. }
                | PublicResolution::RevokedRedirect { .. },
            ) => "group_withheld",
            None => "absent",
        };
        rows.push(vec![
            Expect::text("Public identity candidate withheld"),
            Expect::Empty,
            Expect::text(athlete.id.as_str()),
            Expect::text(&athlete.canonical_name),
            Expect::Empty,
            Expect::Empty,
            Expect::text(format!("retained candidate record; public identity resolution {state}; excluded from accepted people counts; this row does not establish a distinct person")),
        ]);
    }
    Ok(rows)
}
