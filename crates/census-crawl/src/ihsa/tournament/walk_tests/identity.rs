use super::{Harness, Options, TestResult};
use census_domain::model::CentiMetres;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, CanonicalSchool, Grade, Mark,
    SchoolYear, SourceIdentity, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::Table;

fn options() -> Options {
    Options {
        limit: Some(1),
        refresh: false,
        observed_on: "2026-09-20".to_string(),
        states: Vec::new(),
        school_names: Vec::new(),
    }
}

#[test]
fn second_run_resumes_on_the_published_change_signal() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new()?;
            let first = harness
                .run(&Options {
                    limit: Some(1),
                    refresh: false,
                    observed_on: "2026-09-20".to_string(),
                    states: Vec::new(),
                    school_names: Vec::new(),
                })
                .await?;
            check!(eq; first.rows, 20);

            let second = harness
                .run(&Options {
                    limit: Some(1),
                    refresh: false,
                    observed_on: "2026-09-20".to_string(),
                    states: Vec::new(),
                    school_names: Vec::new(),
                })
                .await?;
            check!(eq; second.errors, 0, "notes: {:?}", second.notes);
            check!(eq; second.requests, 0, "a resumed run reads nothing live");
            check!(eq;
                second.from_cache, 4,
                "meets index, terms, and the two lists the archive errors on"
            );
            check!(eq; second.rows, 0, "no summary is read again");
            let notes = second.notes.join("\n");
            check!(notes.contains("0 meets walked, 1 skipped"), "{notes}");
            check!(notes.contains("4 lists already read"), "{notes}");
            check!(
                notes.contains("tournamentId 693 (3A, girls): Archive not available"),
                "{notes}"
            );
            check!(eq;
                harness
                    .scan::<CanonicalPerformance>(Table::Performances)?
                    .len(),
                20,
                "the resumed run appends no second copy"
            );
            Ok(())
        })
}

#[test]
fn out_of_scope_states_spend_no_request() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new()?;
            let scoped = Options {
                states: vec![UsJurisdiction::Minnesota],
                limit: Some(1),
                refresh: false,
                observed_on: "2026-09-20".to_string(),
                school_names: Vec::new(),
            };
            let report = harness.run(&scoped).await?;
            check!(eq; report.requests, 0);
            check!(eq; report.errors, 0);
            check!(
                report
                    .notes
                    .iter()
                    .any(|note| note.contains("does not include IL")),
                "the report names the filter: {:?}",
                report.notes
            );
            check!(eq;
                harness
                    .scan::<CanonicalPerformance>(Table::Performances)?
                    .len(),
                0
            );
            Ok(())
        })
}

#[test]
fn mapped_ids_are_athleticnet_and_ihsa_identity_rows() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let harness = Harness::new()?;
            harness.run(&options()).await?;

            let meets = harness.scan::<CanonicalMeet>(Table::Meets)?;
            let meet = meets.first().ok_or("one meet")?;
            check!(eq; meet.name, "2026 IHSA Boys State Track & Field");
            check!(eq;
                meet.date, "2026-05-28",
                "the earliest event date of the meet"
            );
            check!(eq; meet.end_date.as_deref(), Some("2026-05-30"));
            check!(
                meet.source_identities
                    .iter()
                    .any(|identity| identity.namespace
                        == SourceNamespace::AthleticNet {
                            kind: "live".to_string()
                        }
                        && identity.id == "74003"
                        && identity.url.as_deref()
                            == Some("https://live.athletic.net/meets/74003")),
                "the meet carries its Athletic.net Live id and page: {:?}",
                meet.source_identities
            );

            let athletes = harness.scan::<CanonicalAthlete>(Table::Athletes)?;
            let kinds = |kind: &str| {
                athletes
                    .iter()
                    .filter(|athlete| {
                        athlete.identities().any(|identity| {
                            identity.namespace
                                == SourceNamespace::AthleticNet {
                                    kind: kind.to_string(),
                                }
                        })
                    })
                    .count()
            };
            check!(eq;
                kinds("athlete"),
                68,
                "20 individual finishers + 48 relay legs"
            );
            check!(eq;
                kinds("live"),
                68,
                "every captured finisher publishes both ids"
            );

            let winner = athletes
                .iter()
                .find(|athlete| {
                    athlete.identities().any(|identity| {
                        identity.namespace
                            == SourceNamespace::AthleticNet {
                                kind: "athlete".to_string(),
                            }
                            && identity.id == "27740691"
                    })
                })
                .ok_or("the captured high-jump winner")?;
            check!(eq; winner.canonical_name, "Kehlin Crawford");
            let grade = Grade::new(11).ok_or("grade 11")?;
            let year = SchoolYear::new(2025).ok_or("2025 is a season")?;
            check!(
                winner.observed_grades.iter().any(
                    |observation| observation.grade == grade && observation.school_year == year
                ),
                "grade 11 observed in 2025-26: {:?}",
                winner.observed_grades
            );

            let performances = harness.scan::<CanonicalPerformance>(Table::Performances)?;
            let top = performances
                .iter()
                .find(|performance| performance.mark == Mark::DistanceMetres(CentiMetres::new(202)))
                .ok_or("the captured 2.02m high jump")?;
            check!(eq; top.place, Some(1));
            check!(eq; top.athlete, winner.id);
            check!(eq; top.date, "2026-05-30");
            check!(eq; top.observed_grade, Grade::new(11));
            check!(
                performances
                    .iter()
                    .all(|performance| matches!(performance.mark, Mark::DistanceMetres(_))),
                "a relay's own mark is the team's, so no leg is charged with it"
            );

            let byron = athletes
                .iter()
                .find(|athlete| {
                    athlete.identities().any(|identity| {
                        identity.namespace
                            == SourceNamespace::AssociationAthlete {
                                association: "ihsa".to_string(),
                            }
                            && identity.id == "688:471"
                    })
                })
                .ok_or("the captured boys 1A entry 471")?;
            check!(eq; byron.canonical_name, "Alex Booker");
            let school = harness
                .scan::<CanonicalSchool>(Table::Schools)?
                .into_iter()
                .find(|school| school.id == byron.school)
                .ok_or("the qualifier's school")?;
            check!(eq; school.name, "Byron");
            check!(eq; school.association.as_deref(), Some("ihsa"));
            check!(
                school.source_identities.iter().any(|identity| identity
                    == &SourceIdentity::new(
                        SourceNamespace::AssociationSchool {
                            association: "ihsa".to_string()
                        },
                        "0247"
                    )),
                "the school keeps the association's own id: {:?}",
                school.source_identities
            );
            Ok(())
        })
}
