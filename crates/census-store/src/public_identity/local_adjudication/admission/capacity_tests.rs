use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, TestResult};

fn require_exhausted(
    result: StoreResult<()>,
    resource: PublicIdentityAdmissionResource,
    needed: u64,
    available: u64,
) -> TestResult {
    match result {
        Err(StoreError::PublicIdentity(PublicIdentityError::AdmissionCapacity {
            resource: actual_resource,
            needed: actual_needed,
            available: actual_available,
        })) => {
            check!(eq; actual_resource, resource);
            check!(eq; actual_needed, needed);
            check!(eq; actual_available, available);
            Ok(())
        }
        other => Err(format!("expected admission capacity refusal, got {other:?}").into()),
    }
}

#[test]
fn capacity_accepts_zero_and_exact_limits() -> TestResult {
    require_capacity(0, 0, 0)?;
    require_capacity(MAX_BYTES, MAX_ROOTS, MAX_RESERVATIONS)?;
    Ok(())
}

#[test]
fn byte_capacity_reports_exact_proposed_total_and_limit() -> TestResult {
    let needed = MAX_BYTES.checked_add(1).ok_or("byte test bound")?;
    require_exhausted(
        require_capacity(needed, MAX_ROOTS, MAX_RESERVATIONS),
        PublicIdentityAdmissionResource::Bytes,
        needed,
        MAX_BYTES,
    )
}

#[test]
fn root_capacity_reports_exact_proposed_total_and_limit() -> TestResult {
    let needed = MAX_ROOTS.checked_add(1).ok_or("root test bound")?;
    require_exhausted(
        require_capacity(MAX_BYTES, needed, MAX_RESERVATIONS),
        PublicIdentityAdmissionResource::Roots,
        needed,
        MAX_ROOTS,
    )
}

#[test]
fn reservation_capacity_reports_exact_proposed_total_and_limit() -> TestResult {
    let needed = MAX_RESERVATIONS
        .checked_add(1)
        .ok_or("reservation test bound")?;
    require_exhausted(
        require_capacity(MAX_BYTES, MAX_ROOTS, needed),
        PublicIdentityAdmissionResource::Reservations,
        needed,
        MAX_RESERVATIONS,
    )
}

#[test]
fn capacity_reports_first_exhausted_resource_without_integer_overflow() -> TestResult {
    require_exhausted(
        require_capacity(u64::MAX, u64::MAX, u64::MAX),
        PublicIdentityAdmissionResource::Bytes,
        u64::MAX,
        MAX_BYTES,
    )?;
    require_exhausted(
        require_capacity(MAX_BYTES, u64::MAX, u64::MAX),
        PublicIdentityAdmissionResource::Roots,
        u64::MAX,
        MAX_ROOTS,
    )?;
    require_exhausted(
        require_capacity(MAX_BYTES, MAX_ROOTS, u64::MAX),
        PublicIdentityAdmissionResource::Reservations,
        u64::MAX,
        MAX_RESERVATIONS,
    )
}

#[test]
fn persisted_over_capacity_headers_remain_corruption_without_writes() -> TestResult {
    for resource in [
        PublicIdentityAdmissionResource::Bytes,
        PublicIdentityAdmissionResource::Roots,
        PublicIdentityAdmissionResource::Reservations,
    ] {
        let (_directory, store, writer) = fixture()?;
        writer.enroll_manual_context_admission_v1(&store)?;
        let identity = store.store_identity()?;
        let mut altered = header(&store, &identity)?;
        match resource {
            PublicIdentityAdmissionResource::Bytes => {
                altered.bytes = MAX_BYTES.checked_add(1).ok_or("byte test bound")?;
            }
            PublicIdentityAdmissionResource::Roots => {
                altered.roots = MAX_ROOTS.checked_add(1).ok_or("root test bound")?;
            }
            PublicIdentityAdmissionResource::Reservations => {
                altered.reservations = MAX_RESERVATIONS
                    .checked_add(1)
                    .ok_or("reservation test bound")?;
            }
        }
        store
            .journal
            .insert(HEADER, codec::encode(&altered, header_payload_limit()?)?)?;
        let before = retained_state(&store)?;
        check!(matches!(
            header(&store, &identity),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "manual admission missing, corrupt, or exhausted"
            )))
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}
