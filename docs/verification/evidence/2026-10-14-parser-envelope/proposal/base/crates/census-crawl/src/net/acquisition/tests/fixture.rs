use crate::net::{FetchError, Fetcher};
use crate::{AdapterContext, CaptureContext, ResultsCaptureContext};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{CaptureWriterV1, ResultsCaptureOwnerV1, Store};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::{rustls, TlsAcceptor};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub(crate) const CAPTURED: &str = "2026-10-01T23:44:16Z";
pub(crate) const BODY: &[u8] = include_bytes!("../../../milesplit/owned/fixtures/troy_725218.json");
const CERT: &[u8] = include_bytes!("fixtures/mi-milesplit-public-test-cert.der");
const KEY: &[u8] = include_bytes!("fixtures/mi-milesplit-public-test-key.der");
pub(crate) const HOST: &str = "mi.milesplit.com";

pub(crate) struct Fixture {
    pub(crate) root: tempfile::TempDir,
    pub(crate) store: Store,
    pub(crate) writer: CaptureWriterV1,
    pub(crate) fetcher: Fetcher,
    pub(crate) requests: Arc<AtomicUsize>,
}

impl Fixture {
    pub(crate) fn new(listener: &TcpListener) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        Store::create_capture_qualification_v1(root.path().join("store"))?;
        let (store, writer) =
            Store::open_capture_qualification_v1(root.path().join("store"))?.into_parts();
        let manifest = RunManifest {
            store_identity: writer.store_identity().as_str().to_owned(),
            run: CensusRun::new(SchoolYear::new(2026).ok_or("year")?, 1).ok_or("run")?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Michigan],
        };
        store.bind_run(&manifest)?;
        let mut fetcher = Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            HashMap::new(),
            vec![HOST.into()],
        )?;
        let mut destination =
            crate::net::destination_guard::DestinationGuard::new(vec![HOST.into()]);
        destination.fixture_resolution = Some((HOST.into(), listener.local_addr()?));
        fetcher.destination = Arc::new(destination);
        fetcher.client = crate::net::client::client_builder(Arc::clone(&fetcher.destination), None)
            .tls_certs_only([reqwest::Certificate::from_der(CERT)?])
            .build()?;
        fetcher.capture_clock = crate::net::time::CaptureClock::Injected(Arc::new(FixedClock));
        Ok(Self {
            root,
            store,
            writer,
            fetcher,
            requests: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub(crate) fn reopen(self) -> TestResult<Self> {
        let Self {
            root,
            store,
            writer,
            fetcher,
            requests,
        } = self;
        drop(writer);
        drop(store);
        let (store, writer) =
            Store::open_capture_qualification_v1(root.path().join("store"))?.into_parts();
        Ok(Self {
            root,
            store,
            writer,
            fetcher,
            requests,
        })
    }

    pub(crate) fn owner(
        &self,
        year: u16,
        intent: CaptureCacheIntent,
    ) -> TestResult<ResultsCaptureOwnerV1> {
        let manifest = self.store.run_manifest()?.ok_or("manifest")?;
        let scope = ResultsCaptureScopeV1::new(
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            manifest.run,
            AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
            SelectedHistoryV1::new(HistoryWindow::cohort("2026-10-14")?, year)?,
            SourceAdmissionV1::milesplit_results(&"a".repeat(64))?,
            intent,
        )?;
        Ok(self.writer.results_owner(&self.store, scope)?)
    }

    pub(crate) fn context_owner(
        &self,
        year: u16,
        refresh: bool,
    ) -> TestResult<ResultsCaptureContext> {
        let intent = if refresh {
            CaptureCacheIntent::AcquireNew
        } else {
            CaptureCacheIntent::ReuseEligible
        };
        let owner = self.owner(year, intent)?;
        let grant = self.writer.continuation_for_owned_meet(
            &self.store,
            &owner,
            UsJurisdiction::Michigan,
            "725218",
        )?;
        Ok(ResultsCaptureContext::new(
            owner,
            grant,
            UsJurisdiction::Michigan,
            "725218",
        )?)
    }

    pub(crate) fn context<'a>(
        &'a self,
        owner: &'a ResultsCaptureContext,
        refresh: bool,
    ) -> TestResult<AdapterContext<'a>> {
        Ok(AdapterContext {
            capture: CaptureContext::DurableV1(owner),
            fetcher: &self.fetcher,
            store: &self.store,
            refresh,
            school_year: SchoolYear::new(2026).ok_or("year")?,
            observed_on: "2099-01-01".into(),
            performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-14", "%Y-%m-%d")?,
            recording: None,
        })
    }

    pub(crate) fn reference(&self) -> TestResult<crate::milesplit::ResultSetRef> {
        crate::milesplit::ResultSetRef::parse(
            "https://mi.milesplit.com/meets/725218/results/1266814/raw",
        )
        .ok_or_else(|| "reference".into())
    }

    pub(crate) async fn acquire(
        &self,
        year: u16,
        refresh: bool,
    ) -> TestResult<Result<crate::milesplit::OwnedMeetOutcome, crate::CrawlError>> {
        let owner = self.context_owner(year, refresh)?;
        let ctx = self.context(&owner, refresh)?;
        Ok(
            crate::milesplit::fetch_owned_meet(&ctx, &self.reference()?, &ctx.fetch_options())
                .await,
        )
    }

    pub(crate) fn stop_after_archive(&mut self) {
        self.fetcher.capture_interrupt = super::super::CaptureInterruption::AfterArchive;
    }

    pub(crate) fn resume_capture(&mut self) {
        self.fetcher.capture_interrupt = super::super::CaptureInterruption::None;
    }

    pub(crate) fn count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    pub(crate) fn occurrence(
        outcome: &crate::milesplit::OwnedMeetOutcome,
    ) -> TestResult<&AcquisitionOccurrenceRefV1> {
        match &outcome.provenance {
            crate::net::CaptureProvenance::Retained { occurrence, .. } => Ok(occurrence),
            _ => Err("historical provenance on adopted capture".into()),
        }
    }

    pub(crate) fn cache_paths(&self) -> TestResult<(std::path::PathBuf, std::path::PathBuf)> {
        let url = format!(
            "https://{HOST}/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
            crate::milesplit::OWNED_FIELDS
        );
        Ok(self.fetcher.cache_paths(&Fetcher::key_for("GET", &url, "")))
    }
}

struct FixedClock;
impl census_store::clock::Clock for FixedClock {
    fn today(&self) -> String {
        "2026-10-01".into()
    }
    fn today_iso8601(&self) -> String {
        CAPTURED.into()
    }
    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

pub(crate) enum Mutation<'a> {
    None,
    Metadata(&'a std::path::Path),
    TruncateBody,
}

pub(crate) struct Reply<'a> {
    pub(crate) status: u16,
    pub(crate) body: &'a [u8],
    pub(crate) headers: &'a str,
    pub(crate) before: Mutation<'a>,
}
impl<'a> Reply<'a> {
    pub(crate) fn ok(body: &'a [u8]) -> Self {
        Self {
            status: 200,
            body,
            headers: "ETag: \"capture-v1\"\r\n",
            before: Mutation::None,
        }
    }
}

pub(crate) async fn serve<const N: usize>(
    listener: TcpListener,
    replies: [Reply<'_>; N],
    requests: Arc<AtomicUsize>,
) -> TestResult {
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(CERT.to_vec())],
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(KEY.to_vec())),
    )?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    if N > 8 {
        return Err("fixture exceeds eight HTTP responses".into());
    }
    for reply in replies {
        let (socket, _) =
            tokio::time::timeout(Duration::from_secs(10), listener.accept()).await??;
        let mut stream = acceptor.accept(socket).await?;
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        for _ in 0..64 {
            let count = stream.read(&mut buffer).await?;
            if count == 0 {
                return Err("fixture request ended early".into());
            }
            request.extend_from_slice(buffer.get(..count).ok_or("request size")?);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request = std::str::from_utf8(&request)?;
        check!(request.starts_with("GET /api/v1/meets/725218/performances?isMeetPro=0&fields="));
        check!(request
            .to_ascii_lowercase()
            .contains("host: mi.milesplit.com"));
        requests.fetch_add(1, Ordering::SeqCst);
        if reply.status == 304 {
            check!(request
                .to_ascii_lowercase()
                .contains("if-none-match: \"capture-v1\""));
        }
        let declared_length = match reply.before {
            Mutation::None => reply.body.len(),
            Mutation::Metadata(path) => {
                std::fs::write(path, b"{}")?;
                reply.body.len()
            }
            Mutation::TruncateBody => reply
                .body
                .len()
                .checked_add(1)
                .ok_or("fixture length overflow")?,
        };
        let headers = format!("HTTP/1.1 {} Captured\r\nContent-Length: {}\r\nContent-Type: application/json\r\n{}Connection: close\r\n\r\n", reply.status, declared_length, reply.headers);
        stream.write_all(headers.as_bytes()).await?;
        stream.write_all(reply.body).await?;
        stream.shutdown().await?;
    }
    Ok(())
}

pub(crate) fn injected(error: &crate::CrawlError) -> bool {
    matches!(
        error,
        crate::CrawlError::Fetch(FetchError::Capture(
            crate::net::CaptureTransportError::InjectedInterruption
        ))
    )
}
