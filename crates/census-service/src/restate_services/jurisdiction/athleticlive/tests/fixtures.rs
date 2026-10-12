use super::*;

pub(super) const XC: &str = include_str!(
    "../../../../../../census-crawl/tests/fixtures/athleticlive_results/event-doc-2150205.json"
);
const MANIFEST: &str = include_str!(
    "../../../../../../census-crawl/tests/fixtures/athleticlive/durable-ia-manifest.json"
);

pub(super) fn request(
    jurisdiction: UsJurisdiction,
) -> Result<JurisdictionRequest, Box<dyn std::error::Error>> {
    Ok(JurisdictionRequest {
        jurisdiction,
        season: SchoolYear::new(2026).ok_or("season")?,
        revision: Revision(1),
        history: HistoryWindow::cohort("2026-10-12")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    })
}

pub(super) fn manifest(source: &tempfile::TempDir) -> Result<String, Box<dyn std::error::Error>> {
    let path = source
        .path()
        .join("event.json")
        .to_string_lossy()
        .into_owned();
    std::fs::write(&path, XC)?;
    let mut manifest: Value = serde_json::from_str(MANIFEST)?;
    let entry = manifest.pointer_mut("/meets/0").ok_or("manifest meet")?;
    let old_path = entry
        .pointer("/documents/0")
        .and_then(Value::as_str)
        .ok_or("manifest path")?
        .to_string();
    let metadata = entry["captures"]
        .as_object_mut()
        .ok_or("metadata")?
        .remove(&old_path)
        .ok_or("physical producer metadata")?;
    entry["documents"] = json!([path]);
    entry["captures"] = json!({path:metadata});
    let input = source.path().join("manifest.json");
    std::fs::write(&input, manifest.to_string())?;
    Ok(input.to_string_lossy().into_owned())
}

pub(super) fn seed_schools(store: &Store) -> TestResult {
    let document = census_crawl::athleticlive::parse_event_document(
        &census_crawl::athleticlive::event_doc_url(2_150_205),
        XC,
    )?;
    let mut names = std::collections::BTreeSet::new();
    for row in document.rows {
        let athlete = row.athlete.ok_or("fixture athlete")?;
        let team = athlete.team.ok_or("fixture team")?;
        names.insert(team.school_name().ok_or("fixture school")?.to_string());
    }
    for name in names {
        let (school, _) =
            CanonicalSchool::new(UsJurisdiction::Iowa, &name, normalize_name(&name), None);
        store.append(Table::Schools, &school)?;
    }
    Ok(())
}

pub(super) fn state(
    store: &Store,
    request: &JurisdictionRequest,
) -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    Ok(JurisdictionState {
        identity: census_reconcile::identity::WorkflowIdentity::jurisdiction(
            request.jurisdiction,
            request.season,
            request.revision,
        )
        .as_str()
        .into(),
        athleticlive: ledger::bind(store, request)?,
        plan: Some(SourcePlan::of(
            &crate::restate_services::plan::plan(
                request.jurisdiction,
                crate::restate_services::plan::BrowserLaneState::Absent,
            ),
            "fixture".into(),
        )),
        ..JurisdictionState::default()
    })
}
