use super::*;
use crate::admission::{Coordinator, SlotVector, TryResult, WorkClass};
use census_domain::public_identity_history::funding::{
    envelope::HistoryFundingError, HistoryStorageError, OwnedHistoryBytes,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn profile(bytes: u64) -> WorkClass {
    WorkClass::work(
        bytes,
        1,
        SlotVector {
            tasks: 1,
            ..SlotVector::default()
        },
    )
}

fn reserve(coordinator: &Coordinator, bytes: u64) -> TestResult<Reservation> {
    match coordinator.try_reserve_profile(profile(bytes)) {
        TryResult::Granted(reservation) => Ok(reservation),
        TryResult::Deferred(reason) | TryResult::Refused(reason) => Err(reason.into()),
    }
}

#[test]
fn actual_host_permit_remains_charged_until_the_history_budget_is_destroyed() -> TestResult {
    let coordinator = Coordinator::with(8, 1, 1, 1, profile(8))?;
    let initial = coordinator.remaining();
    {
        let memory = reserve(&coordinator, 8)?.into_history_memory()?;
        {
            let mut funding = memory.funding();
            let mut bytes = OwnedHistoryBytes::with_capacity(8, &mut funding)?;
            bytes.extend_from_slice(b"retained")?;
            let text = bytes.into_text()?;
            check!(eq; text.as_text(), "retained");
            check!(eq; coordinator.remainder_bytes(), 0);
            check!(matches!(
                OwnedHistoryBytes::with_capacity(1, &mut funding),
                Err(HistoryStorageError::Funding(HistoryFundingError::NoBytes))
            ));
            drop(text);
            let reused = OwnedHistoryBytes::with_capacity(8, &mut funding)?;
            drop(reused);
        }
        check!(eq; coordinator.remainder_bytes(), 0);
    }
    check!(eq; coordinator.remaining(), initial);
    Ok(())
}

#[test]
fn a_zero_byte_host_permit_cannot_fund_history_and_returns_its_other_resources() -> TestResult {
    let coordinator = Coordinator::with(8, 1, 1, 1, profile(8))?;
    let initial = coordinator.remaining();
    let reservation = reserve(&coordinator, 0)?;
    check!(eq; coordinator.remainder_work(), 0);
    check!(eq; coordinator.remainder_slots(), 0);
    check!(matches!(
        reservation.into_history_memory(),
        Err(DepartureReason::NoBytes)
    ));
    check!(eq; coordinator.remaining(), initial);
    Ok(())
}
