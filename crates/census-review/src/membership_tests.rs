use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, ReviewState, SourceIdentity,
    SourceNamespace, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};

use crate::consensus::tests::support::{audit, batch, client, lane, row, state, TestResult};
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

#[test]
fn shared_nonperson_and_invalid_person_links_cannot_authorize_same_person_advice() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for (namespace, id) in [
                (SourceNamespace::MilesplitSchool, "123"),
                (SourceNamespace::MilesplitTeam, "123"),
                (SourceNamespace::MilesplitAthlete, "abc"),
                (SourceNamespace::MilesplitAthlete, "0"),
                (SourceNamespace::MilesplitAthlete, "001"),
            ] {
                let dir = tempfile::tempdir()?;
                let store = Store::open(dir.path())?;
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
                store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
                let mut case = ReviewCase::pending(
                    ATHLETE_IDENTITY_FAMILY,
                    first.id.as_str(),
                    "Jordan Smith",
                    "shared links are not person ownership",
                );
                case.member_ids = vec![first.id.cast(), second.id.cast()];
                store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
                let reply = batch(&case, "value_proposed", "identity", "same_person");
                let (first, server_a) = lane(vec![reply.clone()])?;
                let (second, server_b) = lane(vec![reply])?;
                let report = run_lanes(
                    &store,
                    &[client(&first)?, client(&second)?],
                    &options(),
                    "nonperson",
                )
                .await?;
                server_a.join().map_err(|_| "first advice panicked")??;
                server_b.join().map_err(|_| "second advice panicked")??;
                check!(eq; report.accepted, 0);
                check!(eq; report.rejected, 1);
                check!(eq; state(&store)?, ReviewState::Retained);
                check!(eq; row(&store)?.member_ids, case.member_ids);
                check!(eq; audit(&store)?["outcome"], "refused");
                let retained = audit(&store)?;
                let mut shared_source_identity = false;
                for fact in retained["packet"]["evidence"].as_array().ok_or("facts")? {
                    if fact["field"] == "flag"
                        && fact["value"]
                            .as_str()
                            .ok_or("fact text")?
                            .starts_with("shared_source_identity:")
                    {
                        shared_source_identity = true;
                        break;
                    }
                }
                check!(!shared_source_identity);
            }
            Ok(())
        })
}

#[test]
fn group_advice_is_not_requested_when_only_two_of_three_members_would_be_reviewed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for membership in [
                "implicit",
                "three",
                "singleton",
                "duplicate",
                "missing_subject",
            ] {
                let dir = tempfile::tempdir()?;
                let store = Store::open(dir.path())?;
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
                store.append_many(Table::Athletes, &rows)?;
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
                store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
                let report = run_lanes(
                    &store,
                    &[
                        client("http://127.0.0.1:18081")?,
                        client("http://127.0.0.1:18082")?,
                    ],
                    &options(),
                    "incomplete",
                )
                .await?;
                check!(eq; report.requested, 1);
                check!(eq; report.failed, 0);
                check!(eq; report.accepted, 0);
                check!(eq; report.unanswered, 1);
                check!(eq; state(&store)?, ReviewState::Retained);
                check!(eq; row(&store)?.member_ids, case.member_ids);
                check!(eq; audit(&store)?["outcome"], "missing_subject");
                check!(eq; audit(&store)?["packet"], serde_json::Value::Null);
                check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 3);
            }
            Ok(())
        })
}

#[test]
fn an_explicit_nonfirst_pair_reviews_a_and_c_without_substituting_b() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
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
            store.append_many(Table::Athletes, &rows)?;
            let mut case = ReviewCase::pending(
                ATHLETE_IDENTITY_FAMILY,
                rows[0].id.as_str(),
                "Jordan Smith",
                "two named candidates",
            );
            case.member_ids = vec![rows[2].id.cast(), rows[0].id.cast()];
            store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
            let reply = batch(&case, "value_proposed", "identity", "different_person");
            let (first, server_a) = lane(vec![reply.clone()])?;
            let (second, server_b) = lane(vec![reply])?;
            let report = run_lanes(
                &store,
                &[client(&first)?, client(&second)?],
                &options(),
                "pair",
            )
            .await?;
            let requests_a = server_a.join().map_err(|_| "first advice panicked")??;
            let requests_b = server_b.join().map_err(|_| "second advice panicked")??;
            check!(eq; report.accepted, 1);
            check!(eq; state(&store)?, ReviewState::Resolved);
            check!(eq; row(&store)?.value, "different_person");
            check!(eq; row(&store)?.member_ids, case.member_ids);
            let retained = audit(&store)?;
            let candidate_ids = retained["packet"]["evidence"]
                .as_array()
                .ok_or("facts")?
                .iter()
                .find(|fact| fact["field"] == "candidate_ids")
                .ok_or("reviewed candidates")?;
            check!(eq; candidate_ids["value"], format!("{}, {}", rows[0].id, rows[2].id));
            for requests in [requests_a, requests_b] {
                check!(eq; requests.len(), 1);
                let content = requests[0]["messages"][1]["content"]
                    .as_str()
                    .ok_or("attributed request")?;
                check!(content.contains(&format!("side_a_id = {}", rows[0].id)));
                check!(content.contains(&format!("side_b_id = {}", rows[2].id)));
                check!(!content.contains(rows[1].id.as_str()));
            }
            let preserved = store.scan::<CanonicalAthlete>(Table::Athletes)?;
            check!(eq; preserved.len(), 3);
            check!(rows.iter().all(|row| preserved.contains(row)));
            check!(store
                .scan::<census_domain::model::AppliedAthleteIdentity>(
                    Table::AthleteIdentityDecisions
                )?
                .is_empty());
            Ok(())
        })
}
