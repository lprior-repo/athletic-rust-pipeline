use super::*;
use crate::net::CapturedFetch;
use crate::participation_evidence::provider_school::OwnedProviderSchoolPacketV1;
use census_store::{
    PreparedPublicSchoolIntakeV1, PublicSchoolIntakePacketV1, Receipt, RetainedCaptureRootV1,
    StoreError,
};

mod leaves;
mod profile;
mod review;
mod source;
use source::verify_school;

struct Held {
    packet: PublicSchoolIntakePacketV1,
    bytes: Vec<u8>,
    receipt: Receipt,
    root: RetainedCaptureRootV1,
    witness: Witness,
}

async fn acquire_pair(
    fixture: &Fixture,
    listener: TcpListener,
    content_type: &'static str,
) -> TestResult<(CapturedPublicDirectory, CapturedFetch)> {
    let acquire = async {
        let directory = fixture.acquire().await?;
        let roster = fixture
            .fetcher
            .get_public_resource_roster(
                &fixture.context(),
                &directory,
                TEAM,
                &FetchOptions::default(),
            )
            .await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((directory, roster))
    };
    let (requests, pair) = tokio::try_join!(
        serve(
            listener,
            [
                Reply::directory(INDEX),
                Reply {
                    content_type,
                    ..roster_reply()
                },
            ]
        ),
        acquire
    )?;
    check!(eq; requests.len(), 2);
    check!(eq; pair.0.directory().capture().body.as_slice(), INDEX);
    check!(eq; pair.1.capture().body.as_slice(), ROSTER);
    exact_hop(
        &pair.1,
        0,
        &format!("{ORIGIN}{PATH}"),
        200,
        ROSTER.len(),
        content_type,
    )?;
    Ok(pair)
}

fn prepare(
    fixture: &Fixture,
    directory: &CapturedPublicDirectory,
    roster: &CapturedFetch,
) -> TestResult<PreparedPublicSchoolIntakeV1> {
    let directory = directory
        .directory()
        .retained_observed()?
        .ok_or("original directory")?;
    let roster = roster.retained_observed()?.ok_or("original roster")?;
    check!(eq; directory.original(), directory.occurrence());
    check!(eq; roster.original(), roster.occurrence());
    Ok(fixture.store.prepare_public_school_intake(
        &fixture.owner,
        directory.original(),
        roster.original(),
    )?)
}

fn mint(
    fixture: &Fixture,
    directory: &CapturedPublicDirectory,
    roster: &CapturedFetch,
) -> TestResult<Held> {
    let prepared = prepare(fixture, directory, roster)?;
    let packet = prepared.packet().clone();
    let bytes = packet.encode()?;
    let owned = OwnedProviderSchoolPacketV1::from_packet(packet.clone());
    verify_school(&owned.read()?)?;
    let first = fixture.store.apply_public_school_intake(prepared)?;
    check!(first.written());
    check!(eq; first.receipt().operation, format!("public_resource_school_intake_v1:{}", packet.reference().as_str()));
    check!(eq; first.receipt().appended, 0);
    check!(!first.receipt().credited);
    let root = leaves::verify_root(&fixture.store, &packet, &bytes)?;
    let held = Witness::capture(&fixture.store)?;
    let repeated = fixture
        .store
        .apply_public_school_intake(prepare(fixture, directory, roster)?)?;
    check!(repeated.repeated());
    check!(eq; repeated.receipt(), first.receipt());
    check!(eq; Witness::capture(&fixture.store)?, held);
    Ok(Held {
        packet,
        bytes,
        receipt: first.receipt().clone(),
        root,
        witness: held,
    })
}

#[tokio::test]
async fn genuine_source_bodies_mint_observed_school_and_cold_replay_without_third_http(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let held = mint(&fixture, &directory, &roster)?;
    fixture.clear_eligible()?;
    clear_roster_eligible(&fixture)?;
    let fixture = fixture.reopen()?;
    let directory = fixture.acquire().await?;
    let roster = fixture
        .fetcher
        .get_public_resource_roster(
            &fixture.context(),
            &directory,
            TEAM,
            &FetchOptions::default(),
        )
        .await?;
    check!(directory.directory().capture().from_cache);
    check!(roster.capture().from_cache);
    let prepared = prepare(&fixture, &directory, &roster)?;
    check!(eq; prepared.packet().encode()?, held.bytes);
    verify_school(&OwnedProviderSchoolPacketV1::from_packet(prepared.packet().clone()).read()?)?;
    let repeated = fixture.store.apply_public_school_intake(prepared)?;
    check!(repeated.repeated());
    check!(eq; repeated.receipt(), &held.receipt);
    check!(eq; leaves::verify_root(&fixture.store, &held.packet, &held.bytes)?, held.root);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    check!(eq; Witness::capture(&fixture.store)?, held.witness);
    Ok(())
}

#[tokio::test]
async fn actual_non_html_roster_header_cannot_mint_a_school_packet() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "application/json").await?;
    let held = Witness::capture(&fixture.store)?;
    check!(matches!(prepare(&fixture, &directory, &roster)
        .err().ok_or("non-HTML observed roster minted")?.downcast_ref::<StoreError>(),
        Some(StoreError::Refused { detail }) if detail == "public school intake HTML headers differ"));
    check!(fixture
        .store
        .journal_keys(
            census_store::RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1.phase()
        )?
        .is_empty());
    check!(eq; Witness::capture(&fixture.store)?, held);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    Ok(())
}

#[tokio::test]
async fn changed_real_packet_body_or_terminal_receipt_cannot_be_decoded_as_proof() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let prepared = prepare(&fixture, &directory, &roster)?;
    leaves::refuse_tampered_packet(prepared.packet())?;
    let packet = prepared.packet().clone();
    fixture.store.apply_public_school_intake(prepared)?;
    let held = Witness::capture(&fixture.store)?;
    leaves::refuse_changed_leaf(&fixture.store, &packet)?;
    check!(eq; Witness::capture(&fixture.store)?, held);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    Ok(())
}
