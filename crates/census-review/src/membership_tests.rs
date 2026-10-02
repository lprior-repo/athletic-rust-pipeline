use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, ReviewState, SourceIdentity,
    SourceNamespace, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};

use crate::consensus::tests::support::{audit, batch, client, lane, row, state};
use crate::{run_lanes, ReviewFamily, ReviewOptions};

fn athlete(school: &census_domain::model::SchoolId, id: &str) -> CanonicalAthlete {
    CanonicalAthlete::new(
        school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, id),
    )
}

fn options() -> ReviewOptions {
    ReviewOptions {
        families: vec![ReviewFamily::AthleteIdentity],
        limit: 10,
        dry_run: false,
    }
}

#[tokio::test]
async fn shared_nonperson_and_invalid_person_links_cannot_authorize_same_person_advice() {
    for (namespace, id) in [
        (SourceNamespace::MilesplitSchool, "123"),
        (SourceNamespace::MilesplitTeam, "123"),
        (SourceNamespace::MilesplitAthlete, "abc"),
        (SourceNamespace::MilesplitAthlete, "0"),
        (SourceNamespace::MilesplitAthlete, "001"),
    ] {
        let dir = tempfile::tempdir().expect("temporary store");
        let store = Store::open(dir.path()).expect("store");
        let school = CanonicalSchool::new(
            census_domain::UsJurisdiction::Wisconsin,
            "Madison West",
            "madison west",
        )
        .0
        .id;
        let mut first = athlete(&school, "1001");
        let mut second = athlete(&school, "1002");
        first.add_identity(SourceIdentity::new(namespace.clone(), id));
        second.add_identity(SourceIdentity::new(namespace, id));
        store
            .append_many(Table::Athletes, &[first.clone(), second.clone()])
            .expect("source evidence");
        let mut case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            first.id.as_str(),
            "Jordan Smith",
            "shared links are not person ownership",
        );
        case.member_ids = vec![first.id.cast(), second.id.cast()];
        store
            .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
            .expect("case");
        let reply = batch(&case, "value_proposed", "identity", "same_person");
        let (first, server_a) = lane(vec![reply.clone()]);
        let (second, server_b) = lane(vec![reply]);
        let report = run_lanes(
            &store,
            &[client(&first), client(&second)],
            &options(),
            "nonperson",
        )
        .await
        .expect("advice retained");
        server_a.join().expect("first advice");
        server_b.join().expect("second advice");
        assert_eq!(report.accepted, 0);
        assert_eq!(report.rejected, 1);
        assert_eq!(state(&store), ReviewState::Retained);
        assert_eq!(row(&store).member_ids, case.member_ids);
        assert_eq!(audit(&store)["outcome"], "refused");
        assert!(!audit(&store)["packet"]["evidence"]
            .as_array()
            .expect("facts")
            .iter()
            .any(|fact| fact["field"] == "flag"
                && fact["value"]
                    .as_str()
                    .expect("fact text")
                    .starts_with("shared_source_identity:")));
    }
}

#[tokio::test]
async fn group_advice_is_not_requested_when_only_two_of_three_members_would_be_reviewed() {
    for membership in [
        "implicit",
        "three",
        "singleton",
        "duplicate",
        "missing_subject",
    ] {
        let dir = tempfile::tempdir().expect("temporary store");
        let store = Store::open(dir.path()).expect("store");
        let school = CanonicalSchool::new(
            census_domain::UsJurisdiction::Wisconsin,
            "Madison West",
            "madison west",
        )
        .0
        .id;
        let rows = [
            athlete(&school, "1001"),
            athlete(&school, "1002"),
            athlete(&school, "1003"),
        ];
        store
            .append_many(Table::Athletes, &rows)
            .expect("three candidates");
        let mut case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            rows[0].id.as_str(),
            "Jordan Smith",
            "three unresolved candidates",
        );
        case.member_ids = match membership {
            "implicit" => Vec::new(),
            "singleton" => vec![rows[0].id.cast()],
            "duplicate" => vec![rows[0].id.cast(), rows[0].id.cast()],
            "missing_subject" => vec![rows[1].id.cast(), rows[2].id.cast()],
            _ => rows.iter().map(|row| row.id.cast()).collect(),
        };
        store
            .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
            .expect("group case");
        let report = run_lanes(
            &store,
            &[
                client("http://127.0.0.1:18081"),
                client("http://127.0.0.1:18082"),
            ],
            &options(),
            "incomplete",
        )
        .await
        .expect("unresolved group");
        assert_eq!(report.requested, 1);
        assert_eq!(report.failed, 0);
        assert_eq!(report.accepted, 0);
        assert_eq!(report.unanswered, 1);
        assert_eq!(state(&store), ReviewState::Retained);
        assert_eq!(row(&store).member_ids, case.member_ids);
        assert_eq!(audit(&store)["outcome"], "missing_subject");
        assert_eq!(audit(&store)["packet"], serde_json::Value::Null);
        assert_eq!(
            store
                .scan::<CanonicalAthlete>(Table::Athletes)
                .expect("preserved candidates")
                .len(),
            3
        );
    }
}

#[tokio::test]
async fn an_explicit_nonfirst_pair_reviews_a_and_c_without_substituting_b() {
    let dir = tempfile::tempdir().expect("temporary store");
    let store = Store::open(dir.path()).expect("store");
    let school = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Madison West",
        "madison west",
    )
    .0
    .id;
    let mut rows = [
        athlete(&school, "1001"),
        athlete(&school, "1002"),
        athlete(&school, "1003"),
    ];
    rows.sort_by(|first, second| first.id.cmp(&second.id));
    store
        .append_many(Table::Athletes, &rows)
        .expect("three candidates");
    let mut case = ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        rows[0].id.as_str(),
        "Jordan Smith",
        "two named candidates",
    );
    case.member_ids = vec![rows[2].id.cast(), rows[0].id.cast()];
    store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
        .expect("pair case");
    let reply = batch(&case, "value_proposed", "identity", "different_person");
    let (first, server_a) = lane(vec![reply.clone()]);
    let (second, server_b) = lane(vec![reply]);
    let report = run_lanes(
        &store,
        &[client(&first), client(&second)],
        &options(),
        "pair",
    )
    .await
    .expect("pair review");
    let requests_a = server_a.join().expect("first advice");
    let requests_b = server_b.join().expect("second advice");
    assert_eq!(report.accepted, 1);
    assert_eq!(state(&store), ReviewState::Resolved);
    assert_eq!(row(&store).value, "different_person");
    assert_eq!(row(&store).member_ids, case.member_ids);
    let retained = audit(&store);
    let candidate_ids = retained["packet"]["evidence"]
        .as_array()
        .expect("facts")
        .iter()
        .find(|fact| fact["field"] == "candidate_ids")
        .expect("reviewed candidates");
    assert_eq!(
        candidate_ids["value"],
        format!("{}, {}", rows[0].id, rows[2].id)
    );
    for requests in [requests_a, requests_b] {
        assert_eq!(requests.len(), 1);
        let content = requests[0]["messages"][1]["content"]
            .as_str()
            .expect("attributed request");
        assert!(content.contains(&format!("side_a_id = {}", rows[0].id)));
        assert!(content.contains(&format!("side_b_id = {}", rows[2].id)));
        assert!(!content.contains(rows[1].id.as_str()));
    }
    let preserved = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("candidates");
    assert_eq!(preserved.len(), 3);
    assert!(rows.iter().all(|row| preserved.contains(row)));
    assert!(store
        .scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)
        .expect("aliases")
        .is_empty());
}
