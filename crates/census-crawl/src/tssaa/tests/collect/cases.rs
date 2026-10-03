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
            check!(report
                .notes
                .iter()
                .any(|note| note.contains("offline and not cached") && note.contains("?id=157")));
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
            check!(eq; run.schools()?, vec![]);
            let run = FixtureRun::new(RETAINED, Some(RETAINED))?;
            let options = Options {
                school_names: vec!["Not an indexed school".to_string()],
                ..Options::default()
            };
            let missing = run.run(&options, None).await?;
            check!(eq; (missing.rows, missing.errors), (0, 1));
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
            check!(report.notes.iter().any(|note| note.contains("?id=1")));
            check!(!report.notes.iter().any(|note| note.contains("?id=3:")));
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
            check!(eq;
                recorded
                    .journal
                    .iter()
                    .map(|entry| entry.key.as_str())
                    .collect::<Vec<_>>(),
                vec!["TN:157"]
            );
            Ok(())
        })
}
