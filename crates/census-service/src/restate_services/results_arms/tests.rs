//! The results stage's tests: the seed rule, which is the only decision this stage makes.
//!
//! The arms are the adapters' own contracts and are tested where they live. What is tested here is
//! which of this run's rows name an Athletic.net meet, because a wrong answer either requests a meet
//! that does not exist there or silently drops meets the run did enumerate.

use census_crawl::net::Fetcher;
use census_crawl::AdapterContext;
use census_domain::model::SourceMeetRef;
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use super::{arm_for, athleticnet_meet_ids, athleticnet_meets, meet_id_in, ResultsArm};
use census_crawl::milesplit::is_results_page;
use census_store::Store;

/// A row as a meet walk writes one, with only the fields the seed rule reads varied.
fn row(source: &str, id: &str, url: &str, jurisdiction: UsJurisdiction) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(source, id),
        source: source.to_string(),
        source_meet_id: id.to_string(),
        jurisdiction,
        season: "outdoor".to_string(),
        year: 2026,
        name: "Example Invitational".to_string(),
        date: Some("2026-05-02".to_string()),
        venue: "Example High School".to_string(),
        results_url: url.to_string(),
        observed_on: "2026-09-23".to_string(),
    }
}

#[test]
fn a_foreign_host_is_never_read_as_an_athleticnet_meet() {
    // A timer's own path can carry a `/meet/<id>` segment, and its number is that provider's id:
    // reading it here would request a meet that does not exist on Athletic.net.
    assert_eq!(
        meet_id_in("https://results.wayzataresults.com/meet/634313"),
        None
    );
    assert_eq!(
        meet_id_in("https://www.athletic.net/TrackAndField/meet/634313/results"),
        Some(634313)
    );
    assert_eq!(meet_id_in("https://athletic.net/meet/634313"), Some(634313));
    assert_eq!(
        meet_id_in("https://Athletic.Net/xc/meet/634313/"),
        Some(634313)
    );
    // A path segment that names no number is not an id, and neither is a URL with no `/meet/` at all.
    assert_eq!(meet_id_in("https://www.athletic.net/xc/meet/unknown"), None);
    assert_eq!(meet_id_in("https://www.athletic.net/xc/634313"), None);
}

#[test]
fn the_seed_reads_both_row_shapes_and_only_this_jurisdiction() {
    let rows = vec![
        // A row Athletic.net's own walk wrote: the id is the row's own key.
        row(
            "athleticnet",
            "634313",
            "https://www.athletic.net/TrackAndField/meet/634313/results",
            UsJurisdiction::Wisconsin,
        ),
        // A row another source published, naming the meet in the URL it read.
        row(
            "wiaa_results",
            "wi-1",
            "https://www.athletic.net/TrackAndField/meet/700001/results",
            UsJurisdiction::Wisconsin,
        ),
        // The same meet seen twice is one request pair.
        row(
            "mshsl",
            "mn-9",
            "https://www.athletic.net/TrackAndField/meet/700001/results",
            UsJurisdiction::Wisconsin,
        ),
        // Another state's meets are not this run's, even when the URL names one.
        row(
            "wiaa_results",
            "wi-2",
            "https://www.athletic.net/TrackAndField/meet/800001/results",
            UsJurisdiction::Minnesota,
        ),
        // A meet another provider published, which this arm must not request.
        row(
            "milesplit",
            "770621",
            "https://oh.milesplit.com/meets/770621/results",
            UsJurisdiction::Wisconsin,
        ),
        // A row whose key is not an id at all: refused rather than guessed.
        row(
            "athleticnet",
            "not-an-id",
            "https://www.athletic.net/meet/900001",
            UsJurisdiction::Wisconsin,
        ),
    ];
    assert_eq!(
        athleticnet_meet_ids(&rows, UsJurisdiction::Wisconsin),
        vec![634_313, 700_001]
    );
    assert!(athleticnet_meet_ids(&rows, UsJurisdiction::Ohio).is_empty());
}

#[test]
fn a_meet_index_walk_is_not_a_results_arm() {
    // The two stages split by what they consume: an index walk publishes meets, a results arm reads
    // the meets this run published. A source whose own index *is* the meet list belongs to the
    // meet-index stage, and the one whose payload is a whole meet's result files belongs here.
    assert_eq!(arm_for("wiaa_results"), None);
    assert_eq!(arm_for("wayzata"), None);
    assert_eq!(arm_for("milesplit"), Some(ResultsArm::MilesplitResults));
    assert_eq!(arm_for("athleticnet"), Some(ResultsArm::AthleticnetMeets));
}

#[test]
fn only_a_milesplit_results_page_is_read_by_the_result_set_arm() {
    assert!(is_results_page(
        "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results"
    ));
    // Host case is not a name: a site that spells its host differently is the same site.
    assert!(is_results_page("https://WI.MileSplit.COM/meets/1/results"));
    // The `/raw` address under that page is a result set, not the page that lists them.
    assert!(!is_results_page(
        "https://oh.milesplit.com/meets/770621-x/results/1321880/raw"
    ));
    // Another provider's page: the selection carries such rows, the arm must not read them.
    assert!(!is_results_page(
        "https://www.wiaawi.org/Results/Track/2026/d1boysstateresults.htm"
    ));
    assert!(!is_results_page(
        "https://www.athletic.net/TrackAndField/meet/634313/results"
    ));
    assert!(!is_results_page("https://oh.milesplit.com/teams"));
    assert!(!is_results_page("not a url"));
}

/// A store and fetcher pair over a fresh directory, for the one decision the arm makes before it
/// reaches the adapter.
fn scratch() -> (tempfile::TempDir, Store, Fetcher) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
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
        observed_on: "2026-09-24".to_string(),
        recording: None,
    }
}

/// A jurisdiction whose rows name no Athletic.net meet has nothing to pull, and that is not a
/// failure.
///
/// The adapter reads an empty `--meets` as its registry route and refuses for lack of an operator
/// registry file, which is not an argument this stage can supply. Measured 2026-09-24: that refusal
/// failed all forty-nine jurisdictions of the previous national run at its results stage, because
/// only the states whose own sources publish Athletic.net links select anything here. An empty
/// selection is the run's coverage, and the zero row count is what records it.
#[tokio::test]
async fn a_selection_that_names_no_meet_is_not_a_pull() {
    let (_dir, store, fetcher) = scratch();
    let rows = vec![row(
        "milesplit",
        "770621",
        "https://oh.milesplit.com/meets/770621/results",
        UsJurisdiction::Alabama,
    )];
    let (meets, report) = athleticnet_meets(
        &context(&store, &fetcher),
        &rows,
        UsJurisdiction::Alabama,
        "2026-09-24",
    )
    .await
    .expect("an empty selection is the run's coverage, not a failure");
    assert_eq!(meets, 0);
    assert_eq!(report.adapter, "athleticnet");
    assert_eq!(report.unit, "performances");
    assert_eq!(report.rows, 0);
    assert_eq!(report.requests, 0, "nothing to pull means no request");
}
