use super::*;
use census_domain::model::{CanonicalSchool, CanonicalTeam};
use std::time::Duration;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) path: std::path::PathBuf,
    pub(super) reference: String,
    pub(super) school: CanonicalSchool,
    pub(super) program: CanonicalTeam,
    pub(super) reviewed_at: String,
}

impl Fixture {
    pub(super) async fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("store");
        invoke(&path, &["capture-protocol", "init"])?;
        invoke(&path, &["capture-protocol", "enable-admission"])?;
        let (store, writer) = Store::open_current_qualification_v3(&path)?.into_parts();
        let fetcher = Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            Default::default(),
            Vec::new(),
        )?
        .with_origin_locks(root.path().join("origin-locks"))
        .with_offline(true);
        binding::declare(&store, &writer, &fetcher)?;
        let (reference, school, program) = source::mint(&store, &writer, &fetcher).await?;
        store.flush()?;
        drop((fetcher, writer, store));
        let reviewed_at = chrono::Utc::now().to_rfc3339();
        let mut enrollment = vec![
            "capture-protocol",
            "roster-enrollment",
            "--operation",
            "controlled-owned-cli-enrollment",
            "--reviewed-at",
            &reviewed_at,
            "--rationale",
            "Explicit controlled normalized source review fixture",
        ];
        let preview = invoke(&path, &enrollment)?;
        let digest = sha256(&preview);
        enrollment.extend(["--approve-sha256", digest.as_str()]);
        let receipt: census_store::Receipt = serde_json::from_slice(&invoke(&path, &enrollment)?)?;
        check!(eq; receipt.appended, 0);
        check!(!receipt.credited);
        Ok(Self {
            _root: root,
            path,
            reference,
            school,
            program,
            reviewed_at,
        })
    }

    pub(super) fn arguments(&self) -> Vec<&str> {
        vec![
            "capture-protocol",
            "roster-review",
            "--packet-ref",
            &self.reference,
            "--school",
            self.school.id.as_str(),
            "--program",
            self.program.id.as_str(),
            "--operation",
            OPERATION,
            "--reviewed-at",
            &self.reviewed_at,
            "--rationale",
            RATIONALE,
        ]
    }
}

pub(super) fn checkpoint(path: &std::path::Path) -> TestResult<(u64, u64)> {
    let (store, writer) = Store::open_current_qualification_v3(path)?.into_parts();
    let state = (store.snapshot().sequence(), store.receipt_count()?);
    let identity = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("identity capability")?;
    check!(identity.public_ids().next().is_none());
    check!(identity.history_records().is_empty());
    drop((writer, store));
    Ok(state)
}
