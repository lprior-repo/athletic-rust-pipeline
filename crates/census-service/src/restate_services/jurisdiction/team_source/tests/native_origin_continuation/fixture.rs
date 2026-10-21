use super::{request, NativeNode, TestResult};
use crate::restate_services::jurisdiction::{JurisdictionCensus, TeamsSource};
use crate::restate_services::wire::{NationalRequestBinding, TeamsSourceRequest};
use crate::restate_services::{CaptureExecution, Jobs, NationalCensus};
use census_crawl::net::{origin_lock_file_name, FetchError, Fetcher};
use census_domain::model::{CensusRun, GradYear, Revision, RunManifest};
use census_reconcile::identity::WorkflowIdentity;
use census_store::{clock::SystemClock, Store};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

pub(super) const ORIGIN: &str = "https://oh.milesplit.com";
const URL: &str = "https://oh.milesplit.com/teams";
const BODY: &str = "<html><head><meta name='application-name' content='MileSplit'></head><body><table><tbody><tr><td><a href=\"https://oh.milesplit.com/teams/10002-controlled-canary\">Controlled Canary</a></td><td>Mason, OH, USA</td></tr></tbody></table></body></html>";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) store: Arc<Store>,
    pub(super) region: Arc<crate::spawn::Spawner>,
    pub(super) request: TeamsSourceRequest,
    pub(super) initial: Arc<Fetcher>,
    pub(super) continued: Arc<Fetcher>,
    pub(super) owner: JurisdictionCensus,
}

impl Fixture {
    pub(super) async fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let store = Arc::new(Store::open(root.path().join("store"))?);
        let mut request = request()?;
        request.source = "milesplit".into();
        store.bind_run(&RunManifest {
            store_identity: store.store_identity()?.as_str().into(),
            run: CensusRun::new(
                request.jurisdiction.season,
                request.jurisdiction.revision.get(),
            )
            .ok_or("native fixture run identity invalid")?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![request.jurisdiction.jurisdiction],
        })?;
        let region = Arc::new(crate::spawn::Spawner::new());
        let owner = JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::new(SystemClock),
            None,
            CaptureExecution::LegacyFresh,
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
        let national = super::super::bound_national_request(&self.request)?;
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

    pub(super) async fn install_retained_index(&self) -> TestResult {
        let mut hasher = Sha256::new();
        hasher.update(b"GET\x1f");
        hasher.update(URL.as_bytes());
        hasher.update([0x1f]);
        let key: String = hasher
            .finalize()
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let cache = self.continued.cache_dir();
        std::fs::create_dir_all(cache)?;
        std::fs::write(cache.join(format!("{key}.body")), BODY)?;
        std::fs::write(
            cache.join(format!("{key}.meta.json")),
            serde_json::to_vec(&serde_json::json!({
                "url": URL, "response_url": URL, "method": "GET", "status": 200,
                "content_digest": format!("{:x}", Sha256::digest(BODY.as_bytes())),
                "bytes": BODY.len(), "fetched_at": "2026-10-02T12:00:00Z",
                "content_type": "text/html; charset=utf-8"
            }))?,
        )?;
        *self.owner.fetcher.lock().await = Some((Vec::new(), 1, Arc::clone(&self.continued)));
        Ok(())
    }
}
