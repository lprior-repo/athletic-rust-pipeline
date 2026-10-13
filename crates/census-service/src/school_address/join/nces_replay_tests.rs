use super::*;
use census_domain::school_directory::DirectoryKey;
use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[test]
fn retained_nces_generation_replays_without_touching_the_original_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (root, ccd_count, pss_count) = replay_input(dir.path())?;
    let store = Store::open(dir.path().join("store"))?;
    let previously_linked = seed_replay(&store, &root)?;
    let overrides = Overrides::from_acquisitions(vec![
        super::Acquisition {
            entry: 0,
            provider: "nces-ccd".to_string(),
            path: String::new(),
            url: "https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip".to_string(),
            retrieved_at: Some("2026-10-04".to_string()),
        },
        super::Acquisition {
            entry: 1,
            provider: "nces-pss".to_string(),
            path: String::new(),
            url: "https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip".to_string(),
            retrieved_at: Some("2026-10-04".to_string()),
        },
    ])?;
    let report = join_generation(
        &store,
        &root.join("school-address"),
        Some(&dir.path().join("out")),
        overrides,
        Mode::DryRun,
    )?;
    println!(
        "retained generation projected without preexisting NCES claims: {}",
        serde_json::to_string(&report.counters)?
    );
    report_gap_reasons(&report, &previously_linked)?;
    let generation = crate::school_address::verify_current(&root.join("school-address"))?;
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    check!(eq; entries.iter().filter(|entry| matches!(entry.key(), DirectoryKey::Nces(_))).count(), ccd_count);
    check!(eq; entries.iter().filter(|entry| matches!(entry.key(), DirectoryKey::Pss(_))).count(), pss_count);
    check!(eq; report.counters.postal_gaps, report.counters.postal_filled.checked_add(report.counters.postal_remaining).ok_or("gap counter overflow")?);
    check!(eq; report.counters.postal_filled, report.counters.ccd_filled.checked_add(report.counters.pss_filled).ok_or("fill counter overflow")?);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.iter().filter(|school| !school.postal_addresses.is_empty()).count(), 0);
    Ok(())
}

fn is_nces(identity: &SourceIdentity) -> bool {
    matches!(
        &identity.namespace,
        SourceNamespace::SchoolDirectory { provider, .. }
            if provider == "nces-ccd" || provider == "nces-pss"
    )
}

fn seed_replay(store: &Store, root: &Path) -> TestResult<BTreeSet<String>> {
    let reader = std::io::BufReader::new(std::fs::File::open(root.join("out/schools.jsonl"))?);
    let mut schools = BTreeMap::new();
    let mut previously_linked = BTreeSet::new();
    for line in reader.lines() {
        let mut school: CanonicalSchool = serde_json::from_str(&line?)?;
        if school.source_identities.iter().any(is_nces) {
            previously_linked.insert(school.id.as_str().to_string());
        }
        school.postal_addresses.clear();
        school
            .source_identities
            .retain(|identity| !is_nces(identity));
        schools.entry(school.id.clone()).or_insert(school);
    }
    store.append_many(Table::Schools, &schools.into_values().collect::<Vec<_>>())?;
    Ok(previously_linked)
}

fn report_gap_reasons(report: &JoinReport, previously_linked: &BTreeSet<String>) -> TestResult {
    let outcomes = std::fs::read_to_string(&report.outcomes)?;
    let mut reasons = BTreeMap::<String, u64>::new();
    let mut dropped_reasons = BTreeMap::<String, u64>::new();
    let mut gap_ids = BTreeSet::new();
    for line in outcomes.lines() {
        let row: OutcomeRow = serde_json::from_str(line)?;
        if !row.postal_gap {
            continue;
        }
        check!(eq; gap_ids.insert(row.school_id.clone()), true);
        if previously_linked.contains(&row.school_id) && row.reason.as_deref() == Some("ambiguous")
        {
            println!(
                "historical link becomes ambiguous: {}",
                serde_json::to_string(&row)?
            );
            check!(matches!(
                row.detail.as_deref(),
                Some("conflicting_city" | "missing_city_corroboration" | "missing_published_city")
            ));
        }
        let reason = row.reason.or(row.detail).ok_or("gap lacks a reason")?;
        if previously_linked.contains(&row.school_id) {
            increment_reason(&mut dropped_reasons, &reason)?;
        }
        increment_reason(&mut reasons, &reason)?;
    }
    println!(
        "retained generation gap reasons: {}",
        serde_json::to_string(&reasons)?
    );
    println!(
        "historical linked school count: {}; historically linked schools now explicit postal gaps: {}",
        previously_linked.len(),
        serde_json::to_string(&dropped_reasons)?
    );
    check!(eq; u64::try_from(gap_ids.len())?, report.counters.postal_remaining);
    Ok(())
}

fn increment_reason(reasons: &mut BTreeMap<String, u64>, reason: &str) -> TestResult {
    let counter = reasons.entry(reason.to_string()).or_default();
    *counter = counter.checked_add(1).ok_or("reason counter overflow")?;
    Ok(())
}

fn replay_input(dir: &Path) -> TestResult<(PathBuf, usize, usize)> {
    if let Ok(root) = std::env::var("NCES_REPLAY_ROOT") {
        return Ok((PathBuf::from(root), 100307, 22385));
    }
    let root = dir.join("fixture");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../census-crawl/tests/fixtures/nces");
    crate::school_address::run(&crate::school_address::SchoolAddressArgs {
        ccd: Some(fixture.join("ccd_sch_029_2526_head.csv")),
        pss: Some(fixture.join("pss2324_pu_head.csv")),
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        association_directory: Vec::new(),
        out: root.join("school-address"),
        baseline: None,
        ledger: None,
        now: Some("2026-10".to_string()),
        geocode: false,
        validate_postal: false,
    })?;
    let verified = crate::school_address::verify_current(&root.join("school-address"))?;
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(verified.artifact("school_directory.json")?)?;
    let entry = entries
        .iter()
        .find(|entry| {
            matches!(entry.key(), DirectoryKey::Nces(_))
                && entry.address().is_some_and(|address| {
                    address.line1().is_some()
                        && address.city().is_some()
                        && address.state().is_some()
                })
        })
        .ok_or("fixture has no CCD address")?;
    let address = entry.address().ok_or("CCD fixture has no address")?;
    let name = entry.name().ok_or("CCD fixture has no name")?.as_str();
    let (school, _) = CanonicalSchool::new(
        address.state().ok_or("CCD fixture has no state")?,
        name,
        normalize_name(name),
        address.city().map(CityName::as_str),
    );
    std::fs::create_dir_all(root.join("out"))?;
    std::fs::write(root.join("out/schools.jsonl"), serde_json::to_vec(&school)?)?;
    Ok((root, 1557, 374))
}
