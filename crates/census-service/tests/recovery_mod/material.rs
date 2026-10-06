use super::{
    context, fetcher_for, ks, milesplit, normalize_name, open_store, AdapterReport,
    CanonicalSchool, CollectOptions, Digest, Evidence, Path, Sha256, SourceRef, Store, Table,
    UsJurisdiction, KS_DIRECTORY_FIXTURE, KS_DIRECTORY_URL, OBSERVED_ON, UNIT_PHASE,
    WI_ROSTER_FIXTURE, WI_TEAMS_FIXTURE,
};

fn seed_cache(cache: &Path, url: &str, body: &str) -> super::TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect();
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    let meta = serde_json::json!({
        "url": url,
        "response_url": null,
        "method": "GET",
        "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-20T00:00:00Z",
        "content_type": "application/json",
    });
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    Ok(())
}

pub(super) fn store_seeded_with_ks(root: &Path) -> super::TestResult<Store> {
    let store = open_store(root)?;
    seed_cache(
        &store.http_cache_dir(),
        KS_DIRECTORY_URL,
        KS_DIRECTORY_FIXTURE,
    )?;
    Ok(store)
}

pub(super) async fn ks_pass(
    store: &Store,
    limit: Option<usize>,
) -> super::TestResult<AdapterReport> {
    let fetcher = fetcher_for(store)?;
    let ctx = context(&fetcher, store, OBSERVED_ON);
    let options = ks::Options {
        limit,
        refresh: false,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    };
    Ok(ks::collect(&ctx, &options).await?)
}

pub(super) fn store_seeded_with_wisconsin(
    root: &Path,
    site: &milesplit::Site,
) -> super::TestResult<Store> {
    let store = open_store(root)?;
    seed_cache(&store.http_cache_dir(), &site.teams_url(), WI_TEAMS_FIXTURE)?;
    let teams = milesplit::parse_team_index(WI_TEAMS_FIXTURE)?;
    for team in &teams {
        let synthetic_body = format!(
            "<!doctype html><html data-fixture=\"synthetic-owned-roster\"><head>\
             <link rel=\"canonical\" href=\"{}/roster\"></head><body>\
             {WI_ROSTER_FIXTURE}</body></html>",
            team.url
        );
        seed_cache(
            &store.http_cache_dir(),
            &format!("{}/roster", team.url),
            &synthetic_body,
        )?;
    }
    Ok(store)
}

pub(super) fn wi_options(limit_per_state: Option<usize>) -> CollectOptions {
    CollectOptions {
        jurisdictions: vec![UsJurisdiction::Wisconsin],
        limit_per_state,
        concurrency: 2,
        state_concurrency: 1,
        refresh: false,
        school_year: census_domain::model::SchoolYear::DEFAULT,
        observed_on: OBSERVED_ON.to_string(),
        revision: std::num::NonZeroU32::MIN,
    }
}

fn unit_school(key: &str, observed_on: &str) -> CanonicalSchool {
    let name = format!("Recovery Unit {key}");
    let (mut school, _id) = CanonicalSchool::new(
        UsJurisdiction::Kansas,
        name.clone(),
        normalize_name(&name),
        None,
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::id("recovery_fixture"),
        observed_on,
    ));
    school
}

pub(super) fn process_unit(store: &Store, key: &str, observed_on: &str) -> super::TestResult {
    store.append(Table::Schools, &unit_school(key, observed_on))?;
    store.journal_done(
        UNIT_PHASE,
        key,
        &serde_json::json!({ "unit": key, "observed_on": observed_on }),
    )?;
    Ok(())
}

pub(super) fn pending_units<'a>(
    store: &Store,
    units: &[&'a str],
) -> super::TestResult<Vec<&'a str>> {
    let done = store.journal_keys(UNIT_PHASE)?;
    Ok(units
        .iter()
        .copied()
        .filter(|unit| !done.contains(*unit))
        .collect())
}

pub(super) fn process_units(store: &Store, units: &[&str], observed_on: &str) -> super::TestResult {
    for unit in units {
        process_unit(store, unit, observed_on)?;
    }
    Ok(())
}
