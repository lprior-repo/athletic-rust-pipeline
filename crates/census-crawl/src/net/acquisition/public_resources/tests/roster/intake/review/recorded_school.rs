use super::*;
use crate::participation_evidence::recorded::{
    read_original_provider_school, read_recorded_provider_school,
};
use census_store::{RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1};

mod refusals;

fn root(store: &Store, held: &Held) -> TestResult<RetainedCaptureRootReadV1> {
    let original = store
        .snapshot()
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
            held.packet.reference().as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("actual minted observed school root")?;
    check!(eq; original.root(), &held.root);
    check!(eq; original.payload().get().as_bytes(), held.bytes);
    Ok(original)
}

fn projection(
    checked: &crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1,
    held: &Held,
) -> TestResult {
    verify_school(checked)?;
    check!(eq; checked.semantic_contract(), crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2);
    check!(eq; checked.reference().as_str(), held.packet.reference().as_str());
    check!(eq; checked.school(), OwnedProviderSchoolPacketV1::from_packet(held.packet.clone()).read()?.school());
    let child = held.packet.child_evidence();
    let proof = checked.directory_proof();
    check!(eq; child.team_id(), TEAM);
    check!(eq; proof.capture_sha256(), crate::net::cache::content_digest(INDEX));
    check!(eq; proof.row_sha256(), child.row_digest().as_str());
    for (actual, expected) in [
        (proof.row(), child.row()),
        (proof.name(), child.name()),
        (proof.city(), child.city()),
        (proof.anchor(), child.anchor()),
    ] {
        check!(eq; actual.ordinal, expected.ordinal());
        check!(eq; actual.byte_offset, expected.byte_offset());
        check!(eq; actual.byte_length, expected.byte_length());
    }
    Ok(())
}

fn observe(store: &Store, held: &Held) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reference =
        crate::participation_evidence::provider_school::ProviderSchoolPacketRefV1::parse(
            held.packet.reference().as_str(),
        )?;
    let recorded = read_recorded_provider_school(&snapshot, &reference, &mut grant)?
        .ok_or("minted observed school not found by recorded reader")?;
    check!(eq; recorded.root(), &held.root);
    projection(recorded.checked(), held)?;
    check!(grant.usage().encoded_bytes() >= held.bytes.len().checked_mul(3).ok_or("decode bytes")?);
    original_projection(&root(store, held)?, held, &mut grant)?;
    Ok(())
}

fn original_projection(
    original: &RetainedCaptureRootReadV1,
    held: &Held,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let before = grant.usage();
    let checked = read_original_provider_school(original, grant)?;
    projection(&checked, held)?;
    check!(eq; grant.usage().encoded_bytes(), before.encoded_bytes()
        .checked_add(held.bytes.len().checked_mul(3).ok_or("decode bytes")?)
        .ok_or("decode total")?);
    check!(eq; grant.usage().traversal_steps(), before.traversal_steps()
        .checked_add(64).ok_or("decode steps")?);
    Ok(())
}

#[tokio::test]
async fn genuine_owned_school_recorded_and_original_readers_preserve_current_cold_and_frozen_rows(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = review_fixture(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let held = mint(&fixture, &directory, &roster)?;
    observe(&fixture.store, &held)?;
    enroll(&fixture.store)?;
    issue(&fixture.store, &held)?;
    let frozen = freeze(&fixture.store, &held)?;
    let fixture = fixture.reopen()?;
    let before = Witness::capture(&fixture.store)?;
    observe(&fixture.store, &held)?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    let Fixture {
        root,
        store,
        writer,
        owner,
        fetcher,
    } = fixture;
    store.flush()?;
    drop((owner, writer, store, fetcher));
    root.close()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed =
        replay_frozen_roster_reviewed_claims_v1(frozen.bytes(), frozen.reference(), &mut grant)?;
    let claims = replayed.claims(&mut grant)?;
    let [claim] = claims else {
        return Err("exactly one frozen observed school claim".into());
    };
    let original = claim.evidence(&mut grant)?.retained_root(&mut grant)?;
    check!(eq; original.root(), &held.root);
    check!(eq; original.payload().get().as_bytes(), held.bytes);
    original_projection(&original, &held, &mut grant)?;
    Ok(())
}
