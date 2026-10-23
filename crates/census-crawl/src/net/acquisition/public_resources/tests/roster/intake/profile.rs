use super::*;
use crate::participation_evidence::provider_target::roster_child::read_owned_roster_profile_child_v1;
use sha2::{Digest, Sha256};

mod capture;

const ATHLETE: &str = "13749840";
const PROFILE_PATH: &str = "/athletes/13749840-brennan-smith";
const PROFILE_BODY: &[u8] = b"<!doctype html><title>Controlled profile capture</title>";

#[tokio::test]
async fn minted_owned_roster_fetches_profile_and_cold_replays_original_without_fourth_http(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (requests, (held, captured)) = tokio::try_join!(
        serve(
            listener,
            [Reply::directory(INDEX), roster_reply(), profile_reply()],
        ),
        acquire_profile(&fixture),
    )?;
    check!(eq; requests.len(), 3);
    check!(eq; requests.last().ok_or("profile request")?.lines().next(),
        Some(format!("GET {PROFILE_PATH} HTTP/1.1").as_str()));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    check!(eq; captured.capture().url, format!("{ORIGIN}{PROFILE_PATH}"));
    check!(eq; captured.capture().body.as_slice(), PROFILE_BODY);
    check!(!captured.capture().from_cache);
    exact_hop(
        &captured,
        0,
        &format!("{ORIGIN}{PROFILE_PATH}"),
        200,
        PROFILE_BODY.len(),
        "text/html",
    )?;
    cold_replay(fixture, &held, &captured).await
}

async fn acquire_profile(fixture: &Fixture) -> TestResult<(Held, CapturedFetch)> {
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
    let held = mint(fixture, &directory, &roster)?;
    checked_witness(&held)?;
    let captured = fixture
        .fetcher
        .get_public_resource_profile(
            &fixture.context(),
            &held.packet,
            ATHLETE,
            &FetchOptions::default(),
        )
        .await?;
    Ok((held, captured))
}

async fn cold_replay(fixture: Fixture, held: &Held, captured: &CapturedFetch) -> TestResult {
    let original = retained(&fixture.store, captured)?;
    let occurrence = captured
        .retained_observed()?
        .ok_or("original owned profile")?
        .occurrence()
        .clone();
    let operation = format!(
        "public_resource_child_profile_v1:{}",
        occurrence.attempt().work_id()
    );
    let receipt = fixture
        .store
        .receipt(&operation)?
        .ok_or("profile admission receipt")?;
    let before = Witness::capture(&fixture.store)?;
    fixture.clear_eligible()?;
    clear_roster_eligible(&fixture)?;
    clear_profile_eligible(&fixture)?;
    let fixture = fixture.reopen()?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    let replayed = fixture
        .fetcher
        .get_public_resource_profile(
            &fixture.context(),
            &held.packet,
            ATHLETE,
            &FetchOptions::default(),
        )
        .await?;
    check!(replayed.capture().from_cache);
    check!(eq; retained(&fixture.store, &replayed)?, original);
    check!(eq; replayed.retained_observed()?.ok_or("replayed profile")?.occurrence(),
        &occurrence);
    check!(eq; fixture.store.receipt(&operation)?, Some(receipt));
    check!(eq; fixture.store.receipt(&held.receipt.operation)?, Some(held.receipt.clone()));
    check!(eq; held.packet.encode()?, held.bytes);
    check!(eq; leaves::verify_root(&fixture.store, &held.packet, &held.bytes)?, held.root);
    checked_witness(held)?;
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[tokio::test]
async fn prepared_but_unminted_roster_cannot_register_or_dispatch_profile() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let prepared = prepare(&fixture, &directory, &roster)?;
    let original = prepared.packet().encode()?;
    let before = Witness::capture(&fixture.store)?;
    let failure = fixture
        .fetcher
        .get_public_resource_profile(
            &fixture.context(),
            prepared.packet(),
            ATHLETE,
            &FetchOptions::default(),
        )
        .await
        .err()
        .ok_or("unminted roster dispatched a profile")?;
    check!(failure
        .to_string()
        .contains("public profile child retained roster absent"));
    check!(eq; prepared.packet().encode()?, original);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[tokio::test]
async fn minted_roster_missing_or_invalid_athlete_refuses_without_third_http_or_writes(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let held = mint(&fixture, &directory, &roster)?;
    let before = Witness::capture(&fixture.store)?;
    for athlete in ["999999999", "", "13749840/extra", " 13749840"] {
        check!(fixture
            .fetcher
            .get_public_resource_profile(
                &fixture.context(),
                &held.packet,
                athlete,
                &FetchOptions::default(),
            )
            .await
            .is_err());
        check!(eq; fixture.fetcher.stats().await.physical_requests(), 2);
        check!(eq; Witness::capture(&fixture.store)?, before);
        check!(eq; held.packet.encode()?, held.bytes);
        check!(eq; fixture.store.receipt(&held.receipt.operation)?, Some(held.receipt.clone()));
    }
    Ok(())
}

fn checked_witness(held: &Held) -> TestResult {
    let packet = OwnedProviderSchoolPacketV1::from_packet(held.packet.clone());
    let checked = read_owned_roster_profile_child_v1(&packet, ATHLETE)?;
    check!(eq; checked.packet_reference().as_str(), held.packet.reference().as_str());
    check!(eq; checked.site(), UsJurisdiction::Michigan);
    check!(eq; checked.athlete().athlete_id, ATHLETE);
    check!(eq; checked.athlete().profile_url, format!("{ORIGIN}{PROFILE_PATH}"));
    let row = checked.row();
    check!(row.ordinal < 20_000);
    check!(row.byte_length > 0);
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or("profile source row end")?;
    let original = ROSTER
        .get(row.byte_offset..end)
        .ok_or("profile source row bounds")?;
    check!(std::str::from_utf8(original)?.contains(&checked.athlete().profile_url));
    check!(eq; checked.row_digest().as_str(), format!("{:x}", Sha256::digest(original)));
    check!(eq; checked.kernel_digest(), held.packet.child_evidence().kernel_digest());
    Ok(())
}

fn profile_reply() -> Reply<'static> {
    Reply {
        path: PROFILE_PATH,
        status: 200,
        body: PROFILE_BODY,
        content_type: "text/html",
        headers: "",
    }
}

fn clear_profile_eligible(fixture: &Fixture) -> TestResult {
    let url = format!("{ORIGIN}{PROFILE_PATH}");
    let request = fixture
        .fetcher
        .public_resource_request(&url, &FetchOptions::default())?;
    let representation =
        crate::net::RepresentationHeaders::from_entries(request.representation_headers().to_vec())?;
    let extra = crate::net::execute::representation::cache_extra(None, &representation);
    let key = Fetcher::key_for("GET", &url, &extra);
    let (body, metadata) = fixture.fetcher.cache_paths(&key);
    std::fs::remove_file(body)?;
    std::fs::remove_file(metadata)?;
    Ok(())
}
