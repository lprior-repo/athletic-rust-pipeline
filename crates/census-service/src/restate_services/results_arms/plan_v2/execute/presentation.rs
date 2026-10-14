use super::super::{diagnostic, wire::Quarantine};
use crate::restate_services::{jobs, results_arms::ResultsSourceRows};
use census_crawl::net::{EligiblePublicationOutcome, EligiblePublicationSkip};
use census_crawl::{CollectionDisposition, ResolutionCounters};
use census_domain::model::{PreparedRecoveryTarget, SourceMeetRef};
use census_store::Receipt;
use restate_sdk::prelude::{HandlerError, TerminalError};
use std::fmt;

pub(super) fn text(value: &impl fmt::Display) -> Result<String, HandlerError> {
    diagnostic::admitted(value).map_err(super::error)
}

pub(super) fn failed(
    meet: &SourceMeetRef,
    value: &impl fmt::Display,
) -> Result<ResultsSourceRows, HandlerError> {
    let detail = text(value)?;
    let mut row =
        super::super::super::failed_source("milesplit", 1, TerminalError::new(detail).into());
    row.unfinished
        .try_reserve_exact(1)
        .map_err(|_| jobs::invariant("failed result locator allocation"))?;
    row.unfinished.push(copy(&meet.results_url)?);
    Ok(row)
}

pub(super) fn note(row: &mut ResultsSourceRows, value: &str) -> Result<(), HandlerError> {
    row.notes
        .try_reserve(1)
        .map_err(|_| jobs::invariant("result diagnostic allocation"))?;
    row.notes.push(copy(value)?);
    Ok(())
}

pub(super) fn identity(
    target: &PreparedRecoveryTarget,
    receipt: &Receipt,
    outcome: &str,
) -> Result<String, HandlerError> {
    text(&format_args!("prepared recovery {outcome}; slot={}; work={}; ordinal={}; prepared={}; receipt_operation={}; receipt_digest={}; receipt_at={}; appended={}; credited={}",
        target.slot_id(), target.attempt().work_id(), target.attempt().ordinal().get(), target.prepared_digest(),
        receipt.operation, receipt.digest, receipt.at, receipt.appended, receipt.credited))
}

pub(super) fn publication(outcome: EligiblePublicationOutcome) -> Result<String, HandlerError> {
    match outcome {
        EligiblePublicationOutcome::Published => text(&"eligible publication: published"),
        EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::NotReusableResponse) => {
            text(&"eligible publication: skipped non-reusable response")
        }
        EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::NonRoundtrippingMetadata) => {
            text(&"eligible publication: skipped non-roundtripping metadata")
        }
        EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::EnvelopeTooLarge) => {
            text(&"eligible publication: skipped oversized envelope")
        }
        EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::NotAdmitted(error)) => {
            text(&format_args!("eligible publication: not admitted: {error}"))
        }
        EligiblePublicationOutcome::RefusedBeforeReplace(error) => text(&format_args!(
            "eligible publication: refused before replacement: {error}"
        )),
        EligiblePublicationOutcome::FailedDuringReplace(error) => text(&format_args!(
            "eligible publication: failed during replacement: {error}"
        )),
    }
}

pub(super) fn quarantines(
    row: &mut ResultsSourceRows,
    values: Vec<Quarantine>,
    pages_read: u64,
) -> Result<(), HandlerError> {
    note(row, &text(&format_args!("meet_pages_read={pages_read}"))?)?;
    for value in values {
        row.errors = row
            .errors
            .checked_add(1)
            .ok_or_else(|| jobs::invariant("quarantine counter overflow"))?;
        if row.disposition != CollectionDisposition::Failed {
            row.disposition = CollectionDisposition::Partial;
        }
        note(
            row,
            &text(&format_args!(
                "quarantined meet page {}: {}",
                value.url, value.reason
            ))?,
        )?;
        row.unfinished
            .try_reserve(1)
            .map_err(|_| jobs::invariant("quarantine locator allocation"))?;
        row.unfinished.push(value.url);
        let counters = row
            .resolution
            .map_or(ResolutionCounters::default(), core::convert::identity);
        let quarantined = counters
            .quarantined
            .checked_add(1)
            .ok_or_else(|| jobs::invariant("quarantine counter overflow"))?;
        row.resolution = Some(ResolutionCounters {
            quarantined,
            ..counters
        });
    }
    Ok(())
}

fn copy(value: &str) -> Result<String, HandlerError> {
    let mut text = String::new();
    text.try_reserve_exact(value.len())
        .map_err(|_| jobs::invariant("result text allocation"))?;
    text.push_str(value);
    Ok(text)
}
