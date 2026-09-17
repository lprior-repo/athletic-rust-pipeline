use super::exclusions::{bio_resource, find_receipt, html_resource, verify_receipt_metadata};
use crate::{
    domain::{
        evidence::{ProfileEvidence, Sport},
        name::{BioIdentityObservation, CanonicalName},
    },
    runtime::acquisition::{ProfileAcquisition, ProfileProbe},
};
use anyhow::{bail, Context, Result};

pub(super) fn verify(probe: &ProfileProbe, acquisition: &ProfileAcquisition) -> Result<()> {
    if !acquisition.complete
        || !acquisition.failures.is_empty()
        || !probe.complete
        || !probe.failures.is_empty()
        || probe.identities.len() != 2
        || probe.profiles.len() != 2
        || probe.operations.len() != 3
        || !probe
            .identities
            .iter()
            .any(|identity| identity.sport == Sport::TrackField)
        || !probe
            .identities
            .iter()
            .any(|identity| identity.sport == Sport::CrossCountry)
    {
        bail!("complete coverage lacks a complete successful initial and full acquisition");
    }
    let full = acquisition
        .profile
        .as_ref()
        .context("complete acquisition has no full profile")?;
    let html = probe
        .html
        .as_ref()
        .context("complete probe has no HTML evidence")?;
    if !acquisition.responses.starts_with(&probe.responses)
        || !acquisition.operations.starts_with(&probe.operations)
        || html.athlete_id != probe.athlete_id
        || html.profile_url.athlete_id() != probe.athlete_id
        || !full.documents.contains(&html.document)
        || probe.profiles.iter().any(|profile| {
            profile.athlete_id != probe.athlete_id
                || profile.name.value != full.name.value
                || profile
                    .documents
                    .iter()
                    .any(|digest| !full.documents.contains(digest))
        })
    {
        bail!("full acquisition does not retain its initial probe evidence");
    }
    let receipt = find_receipt(probe, &html.document, |url| {
        html_resource(url, probe.athlete_id)
    })?;
    verify_receipt_metadata(receipt, "text/html")?;
    probe
        .identities
        .iter()
        .try_for_each(|identity| verify_bio(probe, full, identity))
}

fn verify_bio(
    probe: &ProfileProbe,
    full: &ProfileEvidence,
    identity: &BioIdentityObservation,
) -> Result<()> {
    if identity.athlete_id != probe.athlete_id
        || identity.first.evidence.document != identity.last.evidence.document
        || identity.first.evidence.locator != "/athlete/FirstName"
        || identity.last.evidence.locator != "/athlete/LastName"
        || CanonicalName::parse(identity.first.value.as_str()).is_err()
        || CanonicalName::parse(identity.last.value.as_str()).is_err()
    {
        bail!("complete Bio identity components are missing or misbound");
    }
    let name_matches = identity
        .first
        .value
        .as_str()
        .trim()
        .bytes()
        .chain(*b" ")
        .chain(identity.last.value.as_str().trim().bytes())
        .eq(full.name.value.as_str().bytes());
    let suffix = match identity.sport {
        Sport::TrackField => "/track-and-field",
        Sport::CrossCountry => "/cross-country",
    };
    if !name_matches
        || !probe.profiles.iter().any(|profile| {
            profile.profile_url.as_str().ends_with(suffix)
                && profile.name.evidence.document == identity.first.evidence.document
        })
    {
        bail!("full profile differs from its initial Bio identities");
    }
    let receipt = find_receipt(probe, &identity.first.evidence.document, |url| {
        bio_resource(url, probe.athlete_id, identity.sport)
    })?;
    verify_receipt_metadata(receipt, "application/json")
}
