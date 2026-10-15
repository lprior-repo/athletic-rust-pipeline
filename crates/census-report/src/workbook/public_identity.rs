use crate::export::public_identity::{output::AcceptedIdentityFields, VerifiedPublicIdentityView};
use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeSet;

pub(super) const CSV_HEADERS: [&str; 2] = ["canonical_candidate_id", "identity_resolution"];
pub(super) const SHEET_HEADERS: [&str; 2] = ["Canonical Candidate ID", "Identity Resolution"];

pub(super) enum Identity<'a> {
    Historical(&'a str),
    Selected(AcceptedIdentityFields<'a>),
}

impl<'a> Identity<'a> {
    pub(super) fn of(
        view: Option<&'a VerifiedPublicIdentityView>,
        candidate: &'a str,
    ) -> ReportResult<Self> {
        match view {
            Some(view) => AcceptedIdentityFields::for_candidate(view, candidate)
                .map(Self::Selected)
                .map_err(Into::into),
            None => Ok(Self::Historical(candidate)),
        }
    }

    pub(super) fn athlete_id(&self) -> &str {
        match self {
            Self::Historical(candidate) => candidate,
            Self::Selected(identity) => identity.athlete_id().as_str(),
        }
    }

    pub(super) fn audit(&self) -> ReportResult<impl Iterator<Item = String> + use<>> {
        let fields = match self {
            Self::Historical(_) => None,
            Self::Selected(identity) => Some([
                identity.canonical_candidate_id().to_string(),
                encode(&identity.resolution_audit())?,
            ]),
        };
        Ok(fields.into_iter().flatten())
    }
}

pub(super) fn view(dataset: &ExportDataset) -> ReportResult<Option<&VerifiedPublicIdentityView>> {
    if dataset.selected_profile_id().is_some() {
        dataset.verified_public_identities().map(Some)
    } else {
        Ok(None)
    }
}

pub(super) fn preflight(
    dataset: &ExportDataset,
    athletes: &[CanonicalAthlete],
) -> ReportResult<()> {
    let Some(view) = view(dataset)? else {
        return Ok(());
    };
    athletes.iter().try_fold(BTreeSet::new(), |mut seen, athlete| {
        let identity = AcceptedIdentityFields::for_candidate(view, athlete.id.as_str())?;
        if !seen.insert(identity.athlete_id()) {
            return Err(ReportError::Invariant {
                detail: "selected publication repeats a retained public athlete ID across candidate groups".into(),
            });
        }
        Ok(seen)
    }).map(|_| ())
}

pub(super) fn headers(
    base: &[&'static str],
    view: Option<&VerifiedPublicIdentityView>,
) -> Vec<&'static str> {
    base.iter()
        .copied()
        .chain(view.into_iter().flat_map(|_| CSV_HEADERS))
        .collect()
}

pub(super) fn encode(value: &impl serde::Serialize) -> ReportResult<String> {
    serde_json::to_string(value).map_err(|error| ReportError::Invariant {
        detail: format!("encoding retained public identity: {error}"),
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(in crate::workbook) mod fixture;
