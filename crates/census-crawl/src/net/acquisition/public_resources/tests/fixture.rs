use super::*;
use census_store::CaptureWriterV1;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub(super) use crate::net::acquisition::tests::fixture::{CERT, HOST, KEY};

pub(super) struct Fixture {
    pub(super) root: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) writer: CaptureWriterV1,
    pub(super) owner: PublicResourceCaptureOwnerV1,
    pub(super) fetcher: Fetcher,
}

impl Fixture {
    pub(super) fn new(listener: &TcpListener) -> TestResult<Self> {
        Self::with_robots_options(listener, &FetchOptions::default())
    }

    pub(super) fn with_robots_options(
        listener: &TcpListener,
        robots: &FetchOptions,
    ) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        Store::create_current_qualification_v3(root.path().join("store"))?;
        let (store, writer) =
            Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
        let fetcher = transport(&store, listener)?;
        let binding = binding::declare(&store, &writer, &fetcher, robots)?;
        check!(eq; binding.store, *writer.store_identity());
        let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
        Ok(Self {
            root,
            store,
            writer,
            owner,
            fetcher,
        })
    }

    pub(super) fn context(&self) -> PublicResourceAcquisition<'_> {
        PublicResourceAcquisition::new(&self.store, &self.writer, &self.owner)
    }

    pub(super) async fn acquire(&self) -> Result<CapturedPublicDirectory, FetchError> {
        self.fetcher
            .get_public_resource_directory(&self.context(), &FetchOptions::default())
            .await
    }

    pub(super) fn work(&self, kind: PublicResourceRootKindV1) -> TestResult<AcquisitionWorkV1> {
        Ok(self.owner.for_root(&self.store, kind)?)
    }

    pub(super) fn reopen(self) -> TestResult<Self> {
        let Self {
            root,
            store,
            writer,
            owner,
            fetcher,
        } = self;
        store.flush()?;
        drop((owner, writer, store));
        let (store, writer) =
            Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
        let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
        Ok(Self {
            root,
            store,
            writer,
            owner,
            fetcher,
        })
    }

    pub(super) fn clear_eligible(&self) -> TestResult {
        let kind = PublicResourceRootKindV1::MileSplitTeamDirectory;
        {
            let work = self.work(kind)?;
            let representation = crate::net::RepresentationHeaders::from_entries(
                work.request().representation_headers().to_vec(),
            )?;
            let extra = crate::net::execute::representation::cache_extra(None, &representation);
            let key = Fetcher::key_for("GET", work.request().original_url(), &extra);
            let (body, metadata) = self.fetcher.cache_paths(&key);
            std::fs::remove_file(body)?;
            std::fs::remove_file(metadata)?;
        }
        Ok(())
    }
}

fn transport(store: &Store, listener: &TcpListener) -> TestResult<Fetcher> {
    let mut fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec![HOST.into()],
    )?;
    let mut destination = crate::net::destination_guard::DestinationGuard::new(vec![HOST.into()]);
    destination.fixture_resolution = Some((HOST.into(), listener.local_addr()?));
    fetcher.destination = Arc::new(destination);
    fetcher.client = crate::net::client::client_builder(Arc::clone(&fetcher.destination), None)
        .tls_certs_only([reqwest::Certificate::from_der(CERT)?])
        .build()?;
    Ok(fetcher)
}

pub(super) struct FixedClock(pub(super) chrono::DateTime<chrono::Utc>);
impl census_store::clock::Clock for FixedClock {
    fn today(&self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
    fn today_iso8601(&self) -> String {
        self.0.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}
