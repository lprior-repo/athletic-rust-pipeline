use super::fixture::*;
use crate::net::destination_guard::DestinationGuard;
use crate::net::{CaptureTransportError, FetchError};
use crate::CrawlError;
use census_domain::model::{
    AcquisitionFailureV1, CaptureProtocolVersion, DispatchRequestV1, HopOrdinal,
    ResponseObservationV1,
};
use census_store::{AcquisitionCheckpointV1, AcquisitionTerminal, ImmutableAttemptResult};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::net::TcpListener;

const HELD_HOST: &str = "www.gobound.com";

struct LocalOnlyDns {
    address: SocketAddr,
    unexpected: Arc<AtomicUsize>,
}

impl reqwest::dns::Resolve for LocalOnlyDns {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        if name.as_str() == HOST {
            let address = self.address;
            return Box::pin(async move {
                let addresses: reqwest::dns::Addrs = Box::new(std::iter::once(address));
                Ok(addresses)
            });
        }
        self.unexpected.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(std::io::Error::other("adopted fixture forbids remote DNS").into()) })
    }
}

fn local_fixture(
    listener: &TcpListener,
    protocol: CaptureProtocolVersion,
    held_host: &str,
) -> TestResult<(Fixture, Arc<AtomicUsize>)> {
    let mut fixture = match protocol {
        CaptureProtocolVersion::V1 => Fixture::new(listener)?,
        CaptureProtocolVersion::V2 => Fixture::new_v2(listener)?,
    };
    let grants = vec![HOST.to_string(), held_host.to_string()];
    let mut destination = DestinationGuard::new(grants.clone());
    destination.fixture_resolution = Some((HOST.into(), listener.local_addr()?));
    fixture.fetcher.destination = Arc::new(destination);
    fixture.fetcher.authorized_hosts = grants;
    let unexpected = Arc::new(AtomicUsize::new(0));
    fixture.fetcher.client =
        crate::net::client::client_builder(Arc::clone(&fixture.fetcher.destination), None)
            .tls_certs_only([reqwest::Certificate::from_der(CERT)?])
            .dns_resolver(Arc::new(LocalOnlyDns {
                address: listener.local_addr()?,
                unexpected: Arc::clone(&unexpected),
            }))
            .build()?;
    let locks = fixture.root.path().join("origin-locks");
    let origin = format!("https://{held_host}");
    let fail_closed = locks.join(crate::net::origin_lock_file_name(&origin));
    std::fs::create_dir_all(&fail_closed)?;
    fixture.fetcher = fixture.fetcher.with_origin_locks(locks);
    check!(matches!(
        fixture.fetcher.origin_locks.ensure(&origin),
        Err(FetchError::OriginLockIo { path, .. }) if path == fail_closed
    ));
    Ok((fixture, unexpected))
}

fn unique_fact(fixture: &Fixture, suffix: &str) -> TestResult<serde_json::Value> {
    let phase = fixture.writer.protocol().phase();
    let keys = fixture.store.journal_keys(phase)?;
    let selected: Vec<_> = keys.iter().filter(|key| key.ends_with(suffix)).collect();
    let [key] = selected.as_slice() else {
        return Err(format!("expected one {suffix} fact, got {}", selected.len()).into());
    };
    fixture
        .store
        .journal_payload(phase, key)?
        .ok_or_else(|| format!("missing fact {key}").into())
}

fn assert_refusal_facts(fixture: &Fixture, target: &str) -> TestResult {
    let terminal: ImmutableAttemptResult =
        serde_json::from_value(unique_fact(fixture, "/terminal")?)?;
    check!(eq; terminal.protocol(), fixture.writer.protocol());
    check!(eq; terminal.attempt().ordinal().get(), 1);
    check!(eq; terminal.terminal(), &AcquisitionTerminal::FailedUncaptured {
        checkpoint: AcquisitionCheckpointV1::ResponseObserved { hop: HopOrdinal::new(0)? },
        failure: AcquisitionFailureV1::AdmissionRefused,
    });
    let observed = unique_fact(fixture, "/response")?;
    let response: ResponseObservationV1 =
        serde_json::from_value(observed.get("response").ok_or("response")?.clone())?;
    let armed = unique_fact(fixture, "/armed")?;
    let request: DispatchRequestV1 =
        serde_json::from_value(armed.get("request").ok_or("request")?.clone())?;
    check!(eq; response.status(), 302);
    check!(eq; response.observed_at(), CAPTURED);
    check!(eq; response.response_url(), request.target_url());
    check!(eq; url::Url::parse(response.response_url())?.host_str(), Some(HOST));
    check!(response
        .headers()
        .iter()
        .any(|(name, value)| name == "location" && value == target));
    let keys = fixture
        .store
        .journal_keys(fixture.writer.protocol().phase())?;
    check!(eq; keys.iter().filter(|key| key.ends_with("/reserved")).count(), 1);
    check!(!keys
        .iter()
        .any(|key| key.ends_with("/prepared") || key.ends_with("/result")));
    Ok(())
}

fn receipts(fixture: &Fixture) -> TestResult<Vec<census_store::Receipt>> {
    let mut rows = Vec::new();
    fixture.store.for_each_receipt(64, |receipt| {
        rows.push(receipt);
        Ok(())
    })?;
    Ok(rows)
}

async fn redirect_refusal(protocol: CaptureProtocolVersion, held_host: &str) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let (fixture, unexpected_dns) = local_fixture(&listener, protocol, held_host)?;
    let target = format!("https://{held_host}/retained");
    let headers = format!("Location: {target}\r\n");
    let replies = [Reply {
        status: 302,
        body: b"",
        headers: &headers,
        before: Mutation::None,
    }];
    let fetch = async {
        let outcome = fixture.acquire(2026, true).await?;
        let Err(CrawlError::Fetch(error @ FetchError::Policy { .. })) = outcome else {
            return Err(format!("expected adopted policy refusal, got {outcome:?}").into());
        };
        check!(!error.retryable());
        let FetchError::Policy { detail } = error else {
            return Err("policy variant changed".into());
        };
        check!(eq; Some(detail.as_str()), crate::registry::acquisition_refusal_for_host(held_host));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(
            serve(listener, replies, Arc::clone(&fixture.requests)),
            fetch
        )
    })
    .await??;
    assert_refusal_facts(&fixture, &target)?;
    let phase = protocol.phase();
    let journal = fixture.store.journal_payloads(phase)?;
    let receipts_before = receipts(&fixture)?;
    let fixture = fixture.reopen()?;
    let replay = fixture.acquire(2026, true).await?;
    check!(matches!(
        replay,
        Err(CrawlError::Fetch(FetchError::Capture(
            CaptureTransportError::Settled
        )))
    ));
    check!(eq; fixture.store.journal_payloads(phase)?, journal);
    check!(eq; receipts(&fixture)?, receipts_before);
    check!(eq; fixture.count(), 1);
    let stats = fixture.fetcher.stats().await;
    check!(eq; stats.physical_requests(), 1);
    check!(!stats.per_host.contains_key(held_host));
    check!(fixture.fetcher.access_conditions().await?.is_empty());
    check!(eq; unexpected_dns.load(Ordering::SeqCst), 0);
    assert_refusal_facts(&fixture, &target)
}

#[tokio::test]
async fn gobound_redirect_refusal_preserves_adopted_checkpoint_and_settled_replay() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        redirect_refusal(protocol, HELD_HOST).await?;
    }
    Ok(())
}

async fn cache_replay(protocol: CaptureProtocolVersion) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let (fixture, unexpected_dns) = local_fixture(&listener, protocol, HELD_HOST)?;
    let fetch = async {
        fixture
            .acquire(2025, true)
            .await?
            .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { Box::new(error) })
    };
    let ((), original) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::try_join!(
            serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
            fetch
        )
    })
    .await??;
    check!(matches!(
        original.verdict(),
        crate::milesplit::OwnedMeetVerdict::Parsed(_)
    ));
    let (body, metadata) = fixture.cache_paths()?;
    let pointer = std::fs::read(&metadata)?;
    let retained_receipts = receipts(&fixture)?;
    let mut fixture = fixture.reopen()?;
    fixture.fetcher = fixture.fetcher.with_offline(true);
    let reused = fixture.acquire(2026, false).await??;
    check!(eq; Fixture::occurrence(&reused)?, Fixture::occurrence(&original)?);
    check!(eq; reused.capture().fetched_at, CAPTURED);
    check!(eq; reused.capture().body, BODY);
    check!(reused.capture().from_cache);
    check!(eq; std::fs::read(&metadata)?, pointer);
    check!(eq; std::fs::read(&body)?, BODY);
    check!(eq; receipts(&fixture)?, retained_receipts);
    let phase = protocol.phase();
    let keys = fixture.store.journal_keys(phase)?;
    check!(eq; keys.iter().filter(|key| key.starts_with("slot/")).count(), 2);
    check!(eq; keys.iter().filter(|key| key.ends_with("/terminal")).count(), 1);
    let journal = fixture.store.journal_payloads(phase)?;
    std::fs::remove_file(body)?;
    std::fs::remove_file(metadata)?;
    let fixture = fixture.reopen()?;
    let replay = fixture.acquire(2025, true).await??;
    check!(eq; Fixture::occurrence(&replay)?, Fixture::occurrence(&original)?);
    check!(eq; replay.capture().fetched_at, CAPTURED);
    check!(eq; replay.capture().body, BODY);
    check!(replay.capture().from_cache);
    check!(eq; fixture.store.journal_payloads(phase)?, journal);
    check!(eq; receipts(&fixture)?, retained_receipts);
    check!(eq; fixture.count(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    check!(fixture.fetcher.access_conditions().await?.is_empty());
    check!(eq; unexpected_dns.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test]
async fn gobound_hold_keeps_unrelated_adopted_cache_and_archive_replay_unchanged() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        cache_replay(protocol).await?;
    }
    Ok(())
}

#[tokio::test]
async fn tfrrs_redirect_refusal_preserves_adopted_checkpoint_and_settled_replay() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        redirect_refusal(protocol, "indiana.tfrrs.org").await?;
    }
    Ok(())
}
