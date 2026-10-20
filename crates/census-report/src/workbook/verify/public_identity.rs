use crate::export::public_identity::output::ResolutionAudit;
use crate::export::public_identity::population::IdentityPopulation;
use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};

pub(in crate::workbook) struct Expected {
    pub(in crate::workbook) athlete_id: String,
    pub(in crate::workbook) audit: Option<[String; 2]>,
}

impl Expected {
    pub(in crate::workbook) fn of(
        identities: &IdentityPopulation<'_>,
        candidate: &str,
    ) -> ReportResult<Self> {
        let Some(view) = identities.selected_view() else {
            return Ok(Self {
                athlete_id: candidate.into(),
                audit: None,
            });
        };
        let group = view.group(candidate).ok_or_else(|| absent(candidate))?;
        let public_id = group.public_id().ok_or_else(|| absent(candidate))?;
        let resolution = view
            .candidate_resolution(candidate)
            .ok_or_else(|| absent(candidate))?;
        Ok(Self {
            athlete_id: public_id.as_str().to_owned(),
            audit: Some([
                group.canonical_candidate().to_string(),
                serde_json::to_string(&ResolutionAudit(resolution)).map_err(|error| {
                    ReportError::Invariant {
                        detail: format!("encoding expected retained public resolution: {error}"),
                    }
                })?,
            ]),
        })
    }
}

pub(in crate::workbook) fn headers(
    dataset: &ExportDataset,
    base: &[&'static str],
) -> Vec<&'static str> {
    let additional = dataset
        .selected_profile_id()
        .map(|_| ["canonical_candidate_id", "identity_resolution"]);
    base.iter()
        .copied()
        .chain(additional.into_iter().flatten())
        .collect()
}

pub(in crate::workbook) fn sheet_headers(
    dataset: &ExportDataset,
    base: &[&'static str],
) -> Vec<&'static str> {
    let additional = dataset
        .selected_profile_id()
        .map(|_| ["Canonical Candidate ID", "Identity Resolution"]);
    base.iter()
        .copied()
        .chain(additional.into_iter().flatten())
        .collect()
}

fn absent(candidate: &str) -> ReportError {
    ReportError::Invariant {
        detail: format!(
            "accepted output candidate {candidate} lacks unambiguous retained public identity"
        ),
    }
}
