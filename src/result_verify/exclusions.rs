use crate::{
    domain::{
        evidence::{EvidenceIssue, Sport},
        identity::{AthleteId, EvidenceDigest},
        name::{BioIdentityObservation, CanonicalName, HtmlIdentity, NameExclusion},
    },
    model::SourceRecord,
    profile,
    runtime::{
        acquisition::{ProfileProbe, TeamRequest},
        profile_worker::team::authorized_requests_value,
        protocol::DocumentReceipt,
    },
    store::{ArtifactStore, MAX_DOCUMENT_BYTES},
};
use anyhow::{bail, Context, Result};
use reqwest::Url;
use std::collections::HashMap;

pub(super) fn verify(
    source: &SourceRecord,
    context: &CanonicalName,
    probe: &ProfileProbe,
    store: &ArtifactStore,
) -> Result<NameExclusion> {
    if CanonicalName::from_source(source)?.as_ref() != Some(context) {
        bail!("name exclusion belongs to another original source-name context");
    }
    if !probe.failures.is_empty() || probe.identities.len() != 2 || probe.profiles.len() != 2 {
        bail!("name exclusion lacks successful full initial Bio parses");
    }
    let html = probe
        .html
        .as_ref()
        .context("name exclusion has no HTML evidence")?;
    let witness = NameExclusion::new(
        context.clone(),
        &probe.identities,
        HtmlIdentity {
            athlete_id: html.athlete_id,
            profile_url: &html.profile_url,
            names: &html.identity_hints,
            issues: &html.issues,
            document: &html.document,
        },
    )?;
    if witness.athlete_id() != probe.athlete_id {
        bail!("name exclusion witness belongs to another athlete");
    }
    let (requests, issues) = probe.identities.iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut requests, mut issues), identity| {
            let (next_requests, next_issues) = verify_bio(probe, identity, store)?;
            requests.extend(next_requests);
            issues.extend(next_issues);
            Ok::<_, anyhow::Error>((requests, issues))
        },
    )?;
    verify_html(probe, store)?;
    if requests != probe.requests
        || issue_counts(issues.iter().chain(html.issues.iter()))
            != issue_counts(probe.issues.iter())
    {
        bail!("probe omits or changes retained initial authorization/HTML metadata");
    }
    Ok(witness)
}

fn issue_counts<'a>(
    issues: impl Iterator<Item = &'a EvidenceIssue>,
) -> HashMap<&'a EvidenceIssue, usize> {
    issues.fold(HashMap::new(), |mut counts, issue| {
        *counts.entry(issue).or_default() += 1;
        counts
    })
}

fn verify_bio(
    probe: &ProfileProbe,
    identity: &BioIdentityObservation,
    store: &ArtifactStore,
) -> Result<(Vec<TeamRequest>, Vec<EvidenceIssue>)> {
    let digest = &identity.first.evidence.document;
    let receipt = find_receipt(probe, digest, |url| {
        bio_resource(url, probe.athlete_id, identity.sport)
    })?;
    let bytes = read_receipt(store, receipt, "application/json")?;
    let root: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&bytes).context("decoding retained raw Bio document")?;
    let (profile, observed) =
        profile::parse_bio_value(probe.athlete_id, identity.sport, digest.clone(), &root)
            .context("reparsing complete raw Bio document")?;
    if observed.as_ref() != Some(identity) || !probe.profiles.contains(&profile) {
        bail!("retained Bio identity/profile differs from its complete raw reparse");
    }
    authorized_requests_value(&root, identity.sport, digest)
        .context("reparsing raw Bio authorization metadata")
}

fn verify_html(probe: &ProfileProbe, store: &ArtifactStore) -> Result<()> {
    let html = probe
        .html
        .as_ref()
        .context("name exclusion has no HTML evidence")?;
    let receipt = find_receipt(probe, &html.document, |url| {
        html_resource(url, probe.athlete_id)
    })?;
    let bytes = read_receipt(store, receipt, "text/html")?;
    let reparsed = profile::parse_profile_html(probe.athlete_id, html.document.clone(), &bytes)
        .context("reparsing complete raw profile HTML")?;
    if &reparsed != html {
        bail!("retained HTML evidence differs from its bounded raw reparse");
    }
    Ok(())
}

pub(super) fn find_receipt<'a>(
    probe: &'a ProfileProbe,
    digest: &EvidenceDigest,
    resource_matches: impl Fn(&Url) -> bool,
) -> Result<&'a DocumentReceipt> {
    probe
        .responses
        .iter()
        .find(|receipt| {
            receipt.digest == *digest
                && receipt.http_status == 200
                && Url::parse(&receipt.source_url).is_ok_and(|url| {
                    matches!(url.scheme(), "http" | "https")
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.fragment().is_none()
                        && resource_matches(&url)
                })
        })
        .context("identity witness has no successful ID/sport-bound source receipt")
}

pub(super) fn bio_resource(url: &Url, athlete_id: AthleteId, sport: Sport) -> bool {
    let id = athlete_id.get().to_string();
    url.path() == "/api/v1/AthleteBio/GetAthleteBioData"
        && url.query_pairs().take(4).count() == 3
        && url
            .query_pairs()
            .filter(|(key, _)| key == "athleteId")
            .map(|(_, value)| value)
            .eq([id.as_str()])
        && url
            .query_pairs()
            .filter(|(key, _)| key == "sport")
            .map(|(_, value)| value)
            .eq([sport.api_code()])
        && url
            .query_pairs()
            .filter(|(key, _)| key == "level")
            .map(|(_, value)| value)
            .eq(["0"])
}

pub(super) fn html_resource(url: &Url, athlete_id: AthleteId) -> bool {
    url.path() == format!("/athlete/{}/track-and-field/all", athlete_id.get())
        && url.query().is_none()
}

fn read_receipt(store: &ArtifactStore, receipt: &DocumentReceipt, media: &str) -> Result<Vec<u8>> {
    verify_receipt_metadata(receipt, media)?;
    // ArtifactStore bounds the read before allocation and verifies the content digest.
    let bytes = store
        .get_bytes(&receipt.digest)
        .context("reading hash-verified raw identity evidence")?;
    if u64::try_from(bytes.len())? != receipt.bytes {
        bail!("identity receipt byte count differs from retained raw bytes");
    }
    Ok(bytes)
}

pub(super) fn verify_receipt_metadata(receipt: &DocumentReceipt, media: &str) -> Result<()> {
    let actual_media = receipt.media_type.split(';').next().map(str::trim);
    if receipt.http_status != 200
        || !actual_media.is_some_and(|value| value.eq_ignore_ascii_case(media))
        || receipt.bytes > u64::try_from(MAX_DOCUMENT_BYTES)?
    {
        bail!("identity receipt has invalid status, media type, or document size");
    }
    Ok(())
}
