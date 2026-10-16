use super::{
    PublicIdentityAdmissionResource, PublicIdentityError, StoreResult, MAX_BYTES, MAX_RESERVATIONS,
    MAX_ROOTS,
};

pub(super) fn require_capacity(bytes: u64, roots: u64, reservations: u64) -> StoreResult<()> {
    for (resource, needed, available) in [
        (PublicIdentityAdmissionResource::Bytes, bytes, MAX_BYTES),
        (PublicIdentityAdmissionResource::Roots, roots, MAX_ROOTS),
        (
            PublicIdentityAdmissionResource::Reservations,
            reservations,
            MAX_RESERVATIONS,
        ),
    ] {
        if needed > available {
            return Err(PublicIdentityError::AdmissionCapacity {
                resource,
                needed,
                available,
            }
            .into());
        }
    }
    Ok(())
}
