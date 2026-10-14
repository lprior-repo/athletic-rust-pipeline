use super::FetchError;
use census_store::StoreError;

mod admission;
mod codec;
mod diagnostic;
#[cfg(test)]
pub(in crate::net) mod diagnostic_testing;
mod inventory;
mod paths;
mod previous;
mod publication;
mod quarantine;
pub(in crate::net) mod replace;
mod reservations;
mod restore;
mod shell;
mod stage;
mod stream;

pub(in crate::net) use publication::publish_recovered_eligible_held;
pub(in crate::net) use restore::restore_exact_prepared_held;

pub enum EligiblePublicationOutcome {
    Published,
    Skipped(EligiblePublicationSkip),
    RefusedBeforeReplace(FetchError),
    FailedDuringReplace(FetchError),
}

pub enum EligiblePublicationSkip {
    NotReusableResponse,
    NonRoundtrippingMetadata,
    EnvelopeTooLarge,
    NotAdmitted(StoreError),
}

pub(in crate::net) fn archive_capacity() -> Result<u64, FetchError> {
    Ok(super::archive::admission::archive_capacity()?.get())
}
