use super::{retained::Retained, wire};
use crate::public_identity::{
    codec, paged_generation::FrozenRawObjectSource, qualification, retained::GraphReader,
    PublicIdentityError,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreResult};
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationV3,
};

pub(super) fn replay(
    bytes: &[u8],
    expected_digest: &str,
) -> StoreResult<AcceptedPublicIdentitySnapshot> {
    super::super::bound(bytes.len(), super::super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES)?;
    if !crate::journal::is_object_id(expected_digest)
        || super::super::digest(bytes) != expected_digest
    {
        return Err(PublicIdentityError::Corrupt("frozen graph bundle digest").into());
    }
    let bundle: wire::Bundle = codec::decode("frozen graph bundle", bytes)?;
    bundle.bounds()?;
    let declaration =
        PublicIdentityQualification::V3(codec::decode::<PublicIdentityQualificationV3>(
            "frozen graph declaration",
            bundle.declaration.as_bytes(),
        )?);
    if declaration.store_identity() != &bundle.store_identity
        || bundle.revision.snapshot_sequence == 0
    {
        return Err(PublicIdentityError::Corrupt("frozen graph lineage binding").into());
    }
    let head = codec::head(bundle.head.as_bytes())?;
    let history = codec::history(bundle.history.as_bytes(), &bundle.store_identity, &head)?;
    if bundle.operations.len() != history.records().len()
        || bundle.revision.evidence_generation < head.sequence
    {
        return Err(PublicIdentityError::Corrupt("frozen graph operation inventory").into());
    }
    let retained = Retained::new(bundle.revision, &bundle.operations, bundle.objects)?;
    let source = FrozenRawObjectSource::new(
        &bundle.store_identity,
        &retained.bindings,
        &retained.physical,
    )?;
    retained.verify_inventory(&source)?;
    let reader = GraphReader::new(&retained, &source);
    qualification::graph_witnesses::validate_graph_retained(&reader, &history, &declaration)?;
    Ok(AcceptedPublicIdentitySnapshot::from_qualified_ledger(
        qualification::Ledger {
            declaration,
            history,
        },
        bundle.revision.snapshot_sequence,
    ))
}
