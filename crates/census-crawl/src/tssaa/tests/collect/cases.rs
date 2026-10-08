use super::{page_options, FixtureRun, RETAINED};
use crate::tssaa::Options;
use crate::Recording;
use census_domain::model::SourceObservation;
use census_domain::UsJurisdiction;
use census_store::Table;

#[test]
fn failed_school_detail_is_reported_and_never_marked_complete(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, None)?;
            let report = run.run(&page_options(), None).await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            check!(eq; run.schools()?, vec![]);
            check!(eq;
                run.store
                    .journal_keys(super::super::super::collect::JOURNAL)?
                    .len(),
                0
            );
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; report.unfinished, vec![format!("{}?id=157", crate::tssaa::DIRECTORY_URL)]);
            Ok(())
        })
}

#[test]
fn empty_index_and_missing_requested_names_cannot_fabricate_schools(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(b"source: [ ],", None)?;
            let empty = run.run(&Options::default(), None).await?;
            check!(eq; (empty.rows, empty.errors), (0, 1));
            check!(eq; empty.disposition, crate::CollectionDisposition::Partial);
            check!(eq; empty.unfinished, vec![crate::tssaa::DIRECTORY_URL.to_string()]);
            check!(eq; run.schools()?, vec![]);
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let options = Options {
                school_names: vec!["Not an indexed school".to_string()],
                ..Options::default()
            };
            let missing = run.run(&options, None).await?;
            check!(eq; (missing.rows, missing.errors), (0, 1));
            check!(eq; missing.disposition, crate::CollectionDisposition::Partial);
            check!(eq; missing.unfinished, vec![crate::tssaa::DIRECTORY_URL.to_string()]);
            check!(eq; run.schools()?, vec![]);
            let blank = run
                .run(
                    &Options {
                        school_names: vec![" ".to_string()],
                        ..Options::default()
                    },
                    None,
                )
                .await?;
            check!(eq; (blank.rows, blank.errors), (0, 1));
            check!(eq; run.schools()?, vec![]);
            Ok(())
        })
}

#[test]
fn non_tennessee_and_zero_limit_do_not_fetch_or_emit() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let foreign = run
                .run(
                    &Options {
                        states: vec![UsJurisdiction::Ohio],
                        ..page_options()
                    },
                    None,
                )
                .await?;
            check!(eq; (foreign.rows, foreign.errors, foreign.requests), (0, 0, 0));
            let zero = run
                .run(
                    &Options {
                        limit: Some(0),
                        ..page_options()
                    },
                    None,
                )
                .await?;
            check!(eq; (zero.rows, zero.errors, zero.requests), (0, 0, 0));
            check!(eq; zero.disposition, crate::CollectionDisposition::Partial);
            check!(eq; zero.unfinished, vec![format!("{}?id=157", crate::tssaa::DIRECTORY_URL)]);
            check!(eq; run.schools()?, vec![]);
            Ok(())
        })
}

#[test]
fn source_owned_out_of_state_record_is_not_relabelled_as_tennessee(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, None)?;
            let report = run
                .run(
                    &Options {
                        school_names: vec!["Northpoint Christian School".to_string()],
                        ..Options::default()
                    },
                    None,
                )
                .await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; report.unfinished, vec![crate::tssaa::DIRECTORY_URL.to_string()]);
            check!(eq; run.schools()?, vec![]);
            Ok(())
        })
}

#[test]
fn limit_bounds_attempts_even_when_selected_detail_fails() -> Result<(), Box<dyn std::error::Error>>
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let report = run
                .run(
                    &Options {
                        limit: Some(1),
                        ..Options::default()
                    },
                    None,
                )
                .await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            check!(eq; run.schools()?, vec![]);
            Ok(())
        })
}

#[test]
fn invalid_address_keeps_valid_appointments_but_no_completion_marker(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let text =
                std::str::from_utf8(RETAINED)?.replace("Franklin, TN 37064", "Franklin, TN BADZIP");
            let run = FixtureRun::new(RETAINED, Some(text.as_bytes()))?;
            let report = run.run(&page_options(), None).await?;
            check!(eq; (report.rows, report.errors), (1, 3));
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; report.unfinished, vec![format!("{}?id=157", crate::tssaa::DIRECTORY_URL)]);
            check!(eq; run.coaches()?.len(), 11);
            check!(eq;
                run.schools()?
                    .first()
                    .map(|school| school.postal_addresses.len()),
                Some(0)
            );
            check!(eq;
                run.store
                    .journal_keys(super::super::super::collect::JOURNAL)?
                    .len(),
                0
            );
            let capture_digest = super::content_digest(text.as_bytes());
            let coaches = run.coaches()?;
            check!(coaches
                .iter()
                .all(|coach| coach.evidence.iter().any(|evidence| {
                    evidence
                        .note
                        .as_ref()
                        .is_some_and(|note| note.contains(&capture_digest))
                        && evidence.observed_on == super::CAPTURED
                })));
            let replay = run.run(&page_options(), None).await?;
            check!(eq; (replay.rows, replay.errors), (0, 3));
            check!(eq; replay.unfinished, report.unfinished);
            check!(eq; run.coaches()?, coaches);
            let run = FixtureRun::new(RETAINED, Some(text.as_bytes()))?;
            let recording = Recording::new();
            let recorded_report = run.run(&page_options(), Some(&recording)).await?;
            check!(eq; (recorded_report.rows, recorded_report.errors), (1, 3));
            check!(eq; recorded_report.unfinished, report.unfinished);
            let recorded = recording.drain();
            check!(eq; recorded_coaches(&recorded)?, coaches);
            check!(recorded
                .journal
                .iter()
                .all(|entry| { entry.phase != super::super::super::collect::JOURNAL }));
            Ok(())
        })
}

#[test]
fn recording_retains_owned_observations_without_touching_store(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let recording = Recording::new();
            let report = run.run(&page_options(), Some(&recording)).await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            check!(eq; run.schools()?, vec![]);
            check!(eq; run.coaches()?, vec![]);
            check!(eq;
                run.store
                    .journal_keys(super::super::super::collect::JOURNAL)?
                    .len(),
                0
            );
            check!(eq; run.store.scan::<SourceObservation>(Table::SourceObservations)?, vec![]);
            let recorded = recording.drain();
            let observation = recorded
                .rows
                .iter()
                .find(|batch| batch.table == Table::SourceObservations)
                .and_then(|batch| batch.rows.first())
                .ok_or_else(|| anyhow::anyhow!("school observation"))?;
            let SourceObservation::School(row) = serde_json::from_value(observation.clone())?
            else {
                return Err("wrong source observation object".into());
            };
            check!(eq;
                (
                    row.source_school_id.as_str(),
                    row.observed_name.as_str(),
                    row.observed_on.as_str()
                ),
                ("157", "Page High School", super::super::CAPTURED)
            );
            let direct = FixtureRun::new(RETAINED, Some(RETAINED))?;
            direct.run(&page_options(), None).await?;
            check!(eq; recorded_coaches(&recorded)?, direct.coaches()?);
            let schools: Vec<census_domain::model::CanonicalSchool> = recorded
                .rows
                .iter()
                .filter(|batch| batch.table == Table::Schools)
                .flat_map(|batch| &batch.rows)
                .map(|row| serde_json::from_value(row.clone()))
                .collect::<Result<_, _>>()?;
            let school = schools
                .iter()
                .find(|school| {
                    school
                        .source_identities
                        .iter()
                        .any(|owner| owner.id == "157")
                })
                .ok_or_else(|| anyhow::anyhow!("recorded source-owned school"))?;
            super::assert_postal_claims(school)?;
            let completions: Vec<_> = recorded
                .journal
                .iter()
                .filter(|entry| entry.phase == super::super::super::collect::JOURNAL)
                .map(|entry| entry.payload.clone())
                .collect();
            super::assert_completion(&completions, school)?;
            Ok(())
        })
}

fn recorded_coaches(
    recorded: &crate::Recorded,
) -> anyhow::Result<Vec<census_domain::model::CanonicalCoach>> {
    let mut coaches: Vec<census_domain::model::CanonicalCoach> = recorded
        .rows
        .iter()
        .filter(|batch| batch.table == Table::Coaches)
        .flat_map(|batch| &batch.rows)
        .map(|row| serde_json::from_value(row.clone()))
        .collect::<Result<_, _>>()?;
    coaches.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(coaches)
}
