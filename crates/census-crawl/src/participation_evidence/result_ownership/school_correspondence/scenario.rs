use super::*;
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::provider_target::SourceTargetPacketV1;
use crate::participation_evidence::recorded::*;
use crate::participation_evidence::result_ownership::read_retained_html_result_rows;
use crate::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use crate::Recording;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{serialized_bytes, CanonicalAthlete, CanonicalSchool};
use census_domain::result_school_correspondence::*;
use census_store::{Application, Store, Table};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod approval;
mod attachment;
mod frozen;
mod refusals;
mod review;
mod scope;
mod setup;
mod stale_subject;
mod tfrrs;

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

fn digest(bytes: &[u8]) -> TestResult<AttachmentDigestV1> {
    Ok(AttachmentDigestV1::parse(&format!(
        "{:x}",
        Sha256::digest(bytes)
    ))?)
}

fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}

fn at() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-17T00:00:00Z")?.to_utc())
}

fn scope() -> TestResult<ResultSchoolScopeV1> {
    Ok(ResultSchoolScopeV1 {
        day: ProfileDate::parse("2026-05-15")?,
        sport: Sport::OutdoorTrack,
        category: Gender::Boys,
        year: SchoolYear::from_date("2026-05-15").ok_or("school year")?,
    })
}

#[test]
fn public_original_school_review_revoke_replay_and_cold_reopen() -> TestResult {
    let (directory, store, input) =
        setup::install().map_err(|error| format!("install original sources: {error}"))?;
    attachment::apply(&store, &input)
        .map_err(|error| format!("review source attachments: {error}"))?;
    setup::enroll(&store).map_err(|error| format!("enroll correspondence review: {error}"))?;
    let originals = setup::originals(&store, &input)?;
    let proposal = review::proposal(&store, &input)
        .map_err(|error| format!("build original-school proposal: {error}"))?;
    let (command, receipt) = review::accept(&store, &proposal)
        .map_err(|error| format!("accept school review: {error}"))?;
    let accepted =
        review::observe(&store).map_err(|error| format!("read accepted school: {error}"))?;
    check!(eq; accepted.as_ref(), Some(&input.school));
    let proof =
        frozen::capture(&store).map_err(|error| format!("freeze accepted school: {error}"))?;
    frozen::observe(&proof, &store, &input, Some(&input.school))
        .map_err(|error| format!("replay accepted school: {error}"))?;
    refusals::verify(&store, &input, &proposal)
        .map_err(|error| format!("verify school authority refusals: {error}"))?;
    let prior = review::record(&store)?;
    approval::verify(&store, &proposal)?;
    let stale = refusals::prepare_stale(&store, &proposal)
        .map_err(|error| format!("prepare second school review: {error}"))?;
    setup::refresh_physical(&store, &input)?;
    check!(eq; review::observe(&store)?, Some(input.school.clone()));
    frozen::observe(&proof, &store, &input, Some(&input.school))?;
    review::revoke(&store, &proposal, &prior)
        .map_err(|error| format!("revoke school review: {error}"))?;
    refusals::apply_stale(&store, stale)?;
    check!(eq; review::observe(&store)?, None);
    let before = setup::state(&store)?;
    let replay = review::replay(&store, command.clone())
        .map_err(|error| format!("replay withdrawn school command: {error}"))?;
    check!(eq; replay.receipt(), &receipt);
    check!(replay.repeated());
    check!(eq; setup::state(&store)?, before);
    check!(eq; review::observe(&store)?, None);
    check!(eq; setup::originals(&store, &input)?, originals);
    let revoked =
        frozen::capture(&store).map_err(|error| format!("freeze revoked school: {error}"))?;
    frozen::observe(&revoked, &store, &input, None)
        .map_err(|error| format!("replay revoked school: {error}"))?;
    frozen::observe(&proof, &store, &input, Some(&input.school))?;
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    check!(eq; setup::state(&store)?, before);
    check!(eq; setup::originals(&store, &input)?, originals);
    check!(eq; review::observe(&store)?, None);
    let replay = review::replay(&store, command)?;
    check!(eq; replay.receipt(), &receipt);
    check!(replay.repeated());
    check!(eq; setup::state(&store)?, before);
    frozen::observe(&proof, &store, &input, Some(&input.school))?;
    frozen::observe(&revoked, &store, &input, None)
}
