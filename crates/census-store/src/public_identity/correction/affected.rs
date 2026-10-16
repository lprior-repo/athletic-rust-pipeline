use super::ReviewedIdentityRequestV1;
use crate::public_identity::{
    authority::graph,
    paged_generation::{RawObjectSource, VerifiedRawGraph},
    PublicIdentityError,
};
use crate::StoreResult;
use census_domain::public_identity_history::HistoryProjection;

pub(in crate::public_identity) fn validate<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    previous: &HistoryProjection,
    request: &ReviewedIdentityRequestV1,
) -> StoreResult<()> {
    let (canonical, affected) = match request {
        ReviewedIdentityRequestV1::Merge { .. } => return Ok(()),
        ReviewedIdentityRequestV1::Split {
            canonical,
            affected_record_keys,
            ..
        }
        | ReviewedIdentityRequestV1::Revoke {
            canonical,
            affected_record_keys,
            ..
        } => (canonical, affected_record_keys),
    };
    let members = graph::members(view, previous, canonical)?;
    let expected = graph::record_keys(view, &members)?;
    if affected != &expected {
        return Err(PublicIdentityError::Corrupt("reviewed affected record inventory").into());
    }
    Ok(())
}
