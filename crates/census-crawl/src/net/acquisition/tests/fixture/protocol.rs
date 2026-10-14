use super::*;

impl Fixture {
    pub(super) fn new_with_protocol(
        listener: &TcpListener,
        protocol: CaptureProtocolVersion,
    ) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let (store, writer) = match protocol {
            CaptureProtocolVersion::V1 => {
                Store::create_capture_qualification_v1(root.path().join("store"))?;
                Store::open_capture_qualification_v1(root.path().join("store"))?.into_parts()
            }
            CaptureProtocolVersion::V2 => {
                Store::create_capture_qualification_v2(root.path().join("store"))?;
                Store::open_capture_qualification_v2(root.path().join("store"))?.into_parts()
            }
        };
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
}
