use super::super::{
    invalid, AuthorityTableV2, CaptureBindingV2, CapturePositionV2, FormatBindingV2, RunBindingV2,
};
use crate::{PublicIdentityInputRevisionsV2, Store, StoreResult, StoreSnapshot};

pub(in crate::public_identity) fn capture(
    store: &Store,
    snapshot: &StoreSnapshot<'_>,
) -> StoreResult<CaptureBindingV2> {
    let revisions = match snapshot.public_identity_input_revisions_v2()? {
        PublicIdentityInputRevisionsV2::Disabled => {
            return Err(invalid("explicit input revision enrollment required"))
        }
        PublicIdentityInputRevisionsV2::Enabled(token) => token,
    };
    let mut counters = [0_u64; 6];
    for (counter, table) in counters.iter_mut().zip(AuthorityTableV2::ALL) {
        *counter = revisions.table_revision(table.table())?;
    }
    let format = store.store_format()?;
    if format.migration_target.is_some() {
        return Err(invalid("capture during migration"));
    }
    let format = FormatBindingV2::new(
        format
            .schema_version
            .ok_or_else(|| invalid("missing capture schema"))?,
        format
            .key_format
            .ok_or_else(|| invalid("missing capture key format"))?,
        format
            .lifecycle_revision
            .ok_or_else(|| invalid("missing capture lifecycle"))?,
    )?;
    let run = match snapshot.run_manifest()? {
        Some(manifest) => RunBindingV2::Bound(manifest),
        None => RunBindingV2::Unbound,
    };
    CaptureBindingV2::new(
        snapshot.store_identity()?,
        format,
        run,
        CapturePositionV2::new(
            snapshot.evidence_generation(),
            snapshot.derived_generation(),
            snapshot.sequence(),
            counters,
        ),
    )
}

pub(in crate::public_identity) fn require_inputs(
    original: &CaptureBindingV2,
    current: &CaptureBindingV2,
) -> StoreResult<()> {
    if original.0.store != current.0.store
        || original.0.format != current.0.format
        || original.0.run != current.0.run
        || original.0.table_revisions != current.0.table_revisions
        || original.0.derived_generation != current.0.derived_generation
    {
        return Err(invalid("capture input lineage or revisions moved"));
    }
    Ok(())
}
