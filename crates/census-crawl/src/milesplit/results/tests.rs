//! What the result-set route's unit of work is: one result set's rows and the entry naming it land
//! in one commit, so an interrupted run leaves the set unread for the next run to land.
//!
//! The fixture is the repository's own capture of `RSID 1321880` (47,564 B, 80 rows, two sections,
//! the Beaver Eastern Invite), the same body `milesplit::tests` reads; here it is the HTTP cache's
//! answer, so the route runs end to end with no socket. Its rows are middle-school rows — every
//! `Yr` is 8 — so the mapper mints the meet and its two events and drops all 80 athletes: the tests
//! below assert the rows this capture does mint, and never a count the run reported about itself.

use super::run::Run;
use super::{collect, Accumulator, ResultSetOptions, Stats, ADAPTER, PHASE};
use crate::net::Fetcher;
use crate::AdapterContext;
use census_domain::model::{
    normalize_name, CanonicalEvent, CanonicalMeet, CanonicalSchool, SchoolYear,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::{HashMap, HashSet};

const OH_RAW: &str =
    include_str!("../../../tests/fixtures/milesplit/oh_meet_770621_rs1321880_raw.html");
const OH_RAW_URL: &str =
    "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw";
/// The journal key one result set earns: the meet id and the provider's own result-set id.
const OH_RAW_KEY: &str = "770621/1321880";
const OBSERVED_ON: &str = "2026-09-22";

/// A store and fetcher pair over a fresh directory.
fn scratch() -> (tempfile::TempDir, Store, Fetcher) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    (dir, store, fetcher)
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> AdapterContext<'a> {
    AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).expect("2026 is a season"),
        observed_on: OBSERVED_ON.to_string(),
        recording: None,
    }
}

/// Seed the fetcher's on-disk cache for `url` under the key `Fetcher` derives
/// (`sha256(method \x1f url \x1f body)[..16]`), so the route is driven with no socket.
fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key: String = hasher.finalize()[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let meta = serde_json::json!({
        "url": url,
        "method": "GET",
        "status": 200,
        "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
        "bytes": body.len(),
        "fetched_at": "2026-09-22T03:55:59Z",
    });
    std::fs::create_dir_all(cache_dir).expect("cache dir");
    std::fs::write(
        cache_dir.join(format!("{key}.meta.json")),
        serde_json::to_string(&meta).expect("meta json"),
    )
    .expect("write meta");
    std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("write body");
}

/// The consolidated school the capture's own rows name (`Jackson` is the first section's first
/// row's label), as the index the route resolves labels against.
fn consolidated() -> Vec<CanonicalSchool> {
    vec![CanonicalSchool::new(UsJurisdiction::Ohio, "Jackson", normalize_name("Jackson")).0]
}

/// Write the consolidated index where the route reads it.
fn write_schools(store: &Store, schools: &[CanonicalSchool]) {
    let mut lines = String::new();
    for school in schools {
        lines.push_str(&serde_json::to_string(school).expect("school serializes"));
        lines.push('\n');
    }
    std::fs::create_dir_all(store.out_dir()).expect("out dir");
    std::fs::write(store.out_dir().join("schools.jsonl"), lines).expect("schools written");
}

/// The route's journal entries, as the store holds them.
fn journal(store: &Store) -> HashSet<String> {
    store.journal_keys(PHASE).expect("journal keys")
}

/// A run over the same index and resume set `collect` opens, stopped before the flush.
fn interrupted_run(ctx: &AdapterContext<'_>) -> Run {
    Run {
        index: SchoolIndex::from_schools(&consolidated()),
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        done: journal(ctx.store),
        pending: Vec::new(),
    }
}

/// The unit of work is the result set: its rows and the entry naming it commit together, so a run
/// that stops after reading a set leaves nothing behind and the next run reads that set again and
/// lands it. The discriminating assertions are the store's own rows: an entry written ahead of the
/// rows would make the second run resume the set as done and never write them.
#[tokio::test]
async fn a_result_set_whose_rows_never_landed_is_read_again_by_the_next_run() {
    let (_dir, store, fetcher) = scratch();
    let schools = consolidated();
    write_schools(&store, &schools);
    seed_cache(fetcher.cache_dir(), OH_RAW_URL, OH_RAW);
    let ctx = context(&store, &fetcher);
    let reference = super::super::wire::ResultSetRef::parse(OH_RAW_URL).expect("a /raw URL");

    // The interrupted run: it reads the set, then the run ends before the flush.
    let mut interrupted = interrupted_run(&ctx);
    interrupted.read(&ctx, &reference).await;
    assert_eq!(
        interrupted.stats.result_sets, 1,
        "the set was read: {:?}",
        interrupted.stats.failures
    );
    assert!(
        interrupted.stats.failures.is_empty(),
        "{:?}",
        interrupted.stats.failures
    );
    assert!(
        journal(&store).is_empty(),
        "the walk writes no entry of its own: {:?}",
        journal(&store)
    );
    assert!(
        store
            .scan::<CanonicalMeet>(Table::Meets)
            .expect("meets read")
            .is_empty(),
        "the walk writes no row of its own"
    );
    drop(interrupted);

    // The next run: the same set, read again, with its rows and its entry landing together.
    let report = collect(
        &ctx,
        &ResultSetOptions {
            urls: vec![OH_RAW_URL.to_string()],
        },
    )
    .await
    .expect("the second run completes");
    assert_eq!(report.adapter, ADAPTER);
    assert_eq!(report.rows, 80, "the capture is a set of eighty rows");
    assert_eq!(report.errors, 0, "{}", report.notes.join("\n"));

    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(
        meets.len(),
        1,
        "the second run lands the set's rows instead of resuming a set whose rows never landed"
    );
    assert_eq!(meets[0].name, "Beaver Eastern Invite");
    assert_eq!(meets[0].date, "2026-09-19");
    let events: Vec<CanonicalEvent> = store.scan(Table::Events).expect("events read");
    assert_eq!(events.len(), 2, "the capture publishes two sections");

    let keys = journal(&store);
    assert_eq!(keys.len(), 1, "one entry per result set: {keys:?}");
    assert!(keys.contains(OH_RAW_KEY), "keys: {keys:?}");

    // A third run has a journaled unit to resume, and resumes it: the entry means the rows landed.
    let third = collect(
        &ctx,
        &ResultSetOptions {
            urls: vec![OH_RAW_URL.to_string()],
        },
    )
    .await
    .expect("the third run completes");
    assert_eq!(third.rows, 0, "the set is not read twice");
    assert!(
        third.notes.join("\n").contains(
            "result sets: 0 read (1 request each), 1 already journaled, 0 empty, 0 failed"
        ),
        "{}",
        third.notes.join("\n")
    );
    assert_eq!(
        store
            .scan::<CanonicalMeet>(Table::Meets)
            .expect("meets read")
            .len(),
        1,
        "the tables are not appended twice"
    );
    assert_eq!(journal(&store).len(), 1);
}
