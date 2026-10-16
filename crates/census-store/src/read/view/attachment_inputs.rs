use super::StoreSnapshot;
use crate::{PublicIdentityInputRevisionsV2, StoreResult};

impl StoreSnapshot<'_> {
    pub(crate) fn attachment_input_revision_bytes(&self) -> StoreResult<Vec<u8>> {
        match self.public_identity_input_revisions_v2()? {
            PublicIdentityInputRevisionsV2::Enabled(token) => {
                crate::input_revisions::encode_attachment_token(token)
            }
            PublicIdentityInputRevisionsV2::Disabled => Err(crate::PublicIdentityError::Corrupt(
                "attachment requires enabled input revisions",
            )
            .into()),
        }
    }
}
