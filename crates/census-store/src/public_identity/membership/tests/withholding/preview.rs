use super::*;
use crate::{PublicIdentityError, StoreError};
use census_domain::public_identity_history::{HistoryError, PublicAthleteId};

#[test]
fn preview_authenticates_exact_attachment_and_never_changes_retained_state() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture)?;
    let Fixture { store, writer, .. } = &fixture;
    let state = retained_state(store)?;
    check!(eq; writer.preview_membership_withholding(store, &request.canonical, &request.attachment)?, request.expected_withheld_memberships);
    for field in ["digest", "membership", "sequence"] {
        let mut wrong = request.attachment.clone();
        match field {
            "digest" => wrong.digest = "0".repeat(64),
            "membership" => wrong.membership_id.push_str("-foreign"),
            _ => wrong.sequence = 1,
        }
        check!(matches!(
            writer.preview_membership_withholding(store, &request.canonical, &wrong),
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "preview attachment binding"
            )))
        ));
        check!(eq; retained_state(store)?, state);
    }
    let foreign = PublicAthleteId::parse(&format!("ath_public_v1_{}", "f".repeat(64)))?;
    check!(matches!(
        writer.preview_membership_withholding(store, &foreign, &request.attachment),
        Err(StoreError::PublicIdentity(PublicIdentityError::History(
            HistoryError::Unknown("public ID")
        )))
    ));
    check!(eq; retained_state(store)?, state);
    Ok(())
}

#[test]
fn preview_refuses_previously_withheld_attachment_without_writes() -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture)?;
    let Fixture { store, writer, .. } = &fixture;
    let prepared = writer.prepare_membership_withholding(
        store,
        "preview-revoked",
        request.clone(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let (bytes, digest) = approval(prepared.record())?;
    writer.apply_membership_withholding(store, prepared, bytes, digest)?;
    let state = retained_state(store)?;
    check!(matches!(
        writer.preview_membership_withholding(store, &request.canonical, &request.attachment),
        Err(StoreError::PublicIdentity(PublicIdentityError::History(
            HistoryError::Revocation
        )))
    ));
    check!(eq; retained_state(store)?, state);
    Ok(())
}
