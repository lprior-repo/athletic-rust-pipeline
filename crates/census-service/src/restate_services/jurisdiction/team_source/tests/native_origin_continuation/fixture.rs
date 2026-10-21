use super::{request, NativeNode, TestResult};
use crate::restate_services::jurisdiction::{JurisdictionCensus, TeamsSource};
use crate::restate_services::wire::{NationalRequestBinding, TeamsSourceRequest};
use crate::restate_services::{CaptureExecution, Jobs, NationalCensus};
use census_crawl::net::{origin_lock_file_name, FetchError, Fetcher};
use census_domain::model::Revision;
use census_reconcile::identity::WorkflowIdentity;
use census_store::{clock::SystemClock, CaptureWriterV1, Store};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

pub(super) const ORIGIN: &str = "https://oh.milesplit.com";
const BODY: &str = "<html><head><meta name='application-name' content='MileSplit'></head><body><table><tbody><tr><td><a href=\"https://oh.milesplit.com/teams/10002-controlled-canary\">Controlled Canary</a></td><td>Mason, OH, USA</td></tr></tbody></table></body></html>";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) store: Arc<Store>,
    pub(super) writer: Arc<CaptureWriterV1>,
    pub(super) region: Arc<crate::spawn::Spawner>,
    pub(super) request: TeamsSourceRequest,
    pub(super) initial: Arc<Fetcher>,
    pub(super) continued: Arc<Fetcher>,
    pub(super) owner: JurisdictionCensus,
}

impl Fixture {
    pub(super) async fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        Store::create_current_qualification_v3(root.path().join("store"))?;
        let (store, writer) =
            Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
        let store = Arc::new(store);
        let writer = Arc::new(writer);
        let mut request = request()?;
        request.source = "milesplit".into();
        super::super::public_resources_fixture::bind(&store, &request)?;
        let region = Arc::new(crate::spawn::Spawner::new());
        let owner = JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::new(SystemClock),
            None,
            CaptureExecution::FreshQualification(Arc::clone(&writer)),
            Jobs::new(Arc::clone(&store), Arc::clone(&region)),
        );
        let locks = root.path().join("origin-locks");
        let initial = Arc::new(
            Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::ZERO,
                Default::default(),
                Vec::new(),
            )?
            .with_origin_locks(&locks),
        );
        let continued = Arc::new(
            Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::ZERO,
                Default::default(),
                Vec::new(),
            )?
            .with_origin_locks(&locks)
            .with_offline(true),
        );
        *owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&initial)));
        Ok(Self {
            _root: root,
            store,
            writer,
            region,
            request,
            initial,
            continued,
            owner,
        })
    }

    pub(super) fn source(&self) -> TeamsSource {
        TeamsSource::new(
            self.owner.clone(),
            Jobs::new(Arc::clone(&self.store), Arc::clone(&self.region)),
        )
    }

    pub(super) fn national(&self) -> NationalCensus {
        NationalCensus::new(
            Arc::new(SystemClock),
            Jobs::new(Arc::clone(&self.store), Arc::clone(&self.region)),
        )
    }

    pub(super) async fn seed_national(
        &self,
        client: &reqwest::Client,
        node: &NativeNode,
    ) -> TestResult {
        let national =
            super::super::public_resources_fixture::national(&self.store, &self.request)?;
        let manifest = self
            .store
            .run_manifest()?
            .ok_or("native fixture run absent")?;
        let key = WorkflowIdentity::national(
            manifest.run.season(),
            Revision(manifest.run.revision()),
            &manifest.jurisdictions,
        )
        .to_string();
        let response = client.post(format!("{}/services/NationalCensus/state", node.admin))
            .json(&serde_json::json!({
                "object_key": key, "scope": null, "version": null,
                "new_state": { "national_request_managed_bundle_v2": serde_json::to_vec(&national)? }
            })).send().await?;
        let status = response.status();
        let detail = response.text().await?;
        check!(eq; status, reqwest::StatusCode::ACCEPTED,
            "isolated native national state seed refused: {detail}");
        let expected = serde_json::to_value(NationalRequestBinding::Bound(Box::new(national)))?;
        for attempt in 0..40 {
            let reply = super::transport::call_empty(
                client,
                &node.ingress,
                "NationalCensus",
                &key,
                "request",
                &format!("national-seed-read-{attempt}"),
            )
            .await?;
            if serde_json::from_slice::<serde_json::Value>(&reply.body)? == expected {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err("isolated native national binding was not applied within forty reads".into())
    }

    pub(super) fn hold_origin(&self) -> TestResult<File> {
        let locks = self._root.path().join("origin-locks");
        std::fs::create_dir_all(&locks)?;
        let mut holder = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(locks.join(origin_lock_file_name(ORIGIN)))?;
        holder.try_lock()?;
        holder.write_all(br#"{"pid":424242,"origin":"https://oh.milesplit.com"}"#)?;
        holder.flush()?;
        check!(matches!(
            self.initial.claim_origin(ORIGIN),
            Err(FetchError::OriginHeld { .. })
        ));
        Ok(holder)
    }

    pub(super) async fn install_controlled_owned_index(&self) -> TestResult {
        super::controlled_capture::install(
            &self.store,
            &self.writer,
            self.request.jurisdiction.jurisdiction,
            BODY.as_bytes(),
        )?;
        *self.owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&self.continued)));
        Ok(())
    }
}
