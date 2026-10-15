pub(crate) mod admission;
mod binding;
pub(in crate::public_identity) mod capture;
pub use capture::{RawGenerationCaptureOutcomeV2, RetainedRawGenerationV2};
mod directory;
mod emit;
mod inventory;
mod operation;
pub use operation::RawGenerationOperationIdV2;
mod limits;
mod page;
mod page_decode;
mod raw;
mod records;
mod scalar;
mod seal;
mod wire;

pub use binding::{
    CaptureBindingV2, CapturePositionV2, FormatBindingV2, RunBindingV2, TableBindingV2,
};
pub use directory::{RawBlockReferenceV2, RawPageBlockV2, RawTableManifestV2, RawTableReferenceV2};
pub use inventory::{InventoryV2, ObjectRefV2};
pub use limits::*;
pub use page::{RawPagePositionV2, RawPageReferenceV2, RawPageV2};
pub use raw::RawPhysicalRowV2;
pub use scalar::{AuthorityTableV2, DigestV2, TableRoleV2};
pub use seal::RawGenerationSealV2;

use crate::{StoreError, StoreResult};

fn invalid(detail: &'static str) -> StoreError {
    StoreError::Refused {
        detail: format!("raw public identity generation V2: {detail}"),
    }
}

fn reserve<T>(capacity: usize) -> StoreResult<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| invalid("allocation refused"))?;
    Ok(values)
}

fn add(left: u64, right: u64) -> StoreResult<u64> {
    left.checked_add(right).ok_or(StoreError::CounterOverflow)
}

#[cfg(test)]
mod tests;
