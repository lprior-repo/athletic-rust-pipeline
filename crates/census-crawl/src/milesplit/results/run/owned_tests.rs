use super::*;
use crate::net::Fetcher;
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, Mark, SchoolYear};
use census_store::{Store, Table};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod acquisition_ledger;
mod binding;
mod changed_capture;
mod fixture;
mod mixed_replay;
mod partial_replay;
mod replay;
mod result_sets;
mod unresolved;
use fixture::*;

#[test]
fn legacy_raw_receipts_do_not_authorize_synthetic_owners_or_suppress_owned_projection() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let prior = json!({"disposition":"complete","rows":80});
            for phase in [
                "milesplit_result_sets_v2",
                "milesplit_result_sets_v4",
                super::super::RESULT_SET_PHASE,
            ] {
                store.journal_done(phase, "725218/1266814", &prior)?;
            }
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let report =
                super::super::collect(&context(&store, &fetcher)?, &options(&reference)).await?;
            check!(eq; report.errors, 0, "{:?}", report.notes);
            let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
            check!(eq;
                athletes
                    .iter()
                    .map(|athlete| Ok(athlete.source.as_ref().ok_or("owner")?.id.as_str()))
                    .collect::<TestResult<std::collections::BTreeSet<_>>>()?,
                std::collections::BTreeSet::from(["14222592", "11357806"])
            );
            check!(!athletes
                .iter()
                .any(|athlete| athlete.canonical_name == "Jeydyn Fields"));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let long = performances
                .iter()
                .find(|row| row.source_key == "milesplit_result:201782263")
                .ok_or("original LJ projected instead of raw synthetic rows")?;
            check!(eq;
                long.mark,
                Mark::FieldImperial {
                    feet_mark: "13-9".into(),
                    metres: census_domain::model::CentiMetres::new(419)
                }
            );
            check!(eq; long.evidence[0].observed_on, "2026-10-01T23:44:16Z");
            check!(eq; long.observed_grade, None);
            check!(eq; long.date, "2026-03-27");
            let meets: Vec<census_domain::model::CanonicalMeet> = store.scan(Table::Meets)?;
            check!(eq; meets.len(), 1);
            check!(eq; meets[0].name, "Troy Invitational #2");
            check!(eq; meets[0].date, "2026-03-27");
            check!(eq;
                meets[0].sports,
                vec![census_domain::model::Sport::OutdoorTrack]
            );
            let teams: Vec<census_domain::model::CanonicalTeam> = store.scan(Table::Teams)?;
            let published_season = SchoolYear::new(2025).ok_or("published season")?;
            check!(teams
                .iter()
                .all(|team| team.school_year == published_season));
            for phase in [
                "milesplit_result_sets_v2",
                "milesplit_result_sets_v4",
                super::super::RESULT_SET_PHASE,
            ] {
                check!(eq;
                    store
                        .journal_payload(phase, "725218/1266814")
                        ?,
                    Some(prior.clone())
                );
            }
            let payloads = store.journal_payloads(super::super::RESULT_SET_PHASE)?;
            let receipt = payloads
                .iter()
                .find(|row| row["disposition"] == "projection_applied")
                .ok_or("projection receipt")?;
            check!(eq; receipt["source_completeness"], "unknown");
            check!(eq; receipt["ownership_complete"], false);
            check!(eq; receipt["canonical_identity_accepted"], false);
            check!(eq; receipt["lifetime_pr_claimed"], false);
            Ok(())
        })
}

#[test]
fn relay_source_rows_are_retained_without_individual_membership_or_split_materialization(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
            let relays: serde_json::Value = serde_json::from_slice(RELAYS)?;
            document["data"].as_array_mut().ok_or("rows")?.extend(
                relays["data"]
                    .as_array()
                    .ok_or("relay rows")?
                    .iter()
                    .cloned(),
            );
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&document)?)?;
            seed_metadata(&fetcher, &reference)?;
            let report =
                super::super::collect(&context(&store, &fetcher)?, &options(&reference)).await?;
            check!(eq; report.errors, 0, "{:?}", report.notes);
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                performances
                    .iter()
                    .map(|row| row.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263",
                    "milesplit_result:201782277",
                    "milesplit_result:201782806",
                ])
            );
            let owned = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
            let relay_locators: std::collections::BTreeSet<_> = owned
                .iter()
                .filter(|row| row["kind"] == "team_relay")
                .map(|row| Ok(row["locator"].as_str().ok_or("locator")?))
                .collect::<TestResult<_>>()?;
            check!(eq;
                relay_locators,
                std::collections::BTreeSet::from(["data[3]", "data[4]", "data[5]"])
            );
            let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
            check!(athletes.iter().all(|athlete| athlete
                .source
                .as_ref()
                .is_some_and(|source| { source.id == "14222592" || source.id == "11357806" })));
            Ok(())
        })
}

#[test]
fn a_signed_meet_identifier_cannot_rebind_a_public_owned_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                vec!["milesplit.com".into()],
            )?
            .with_offline(true);
            let mut reference =
                ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
                    .ok_or("reference")?;
            seed_owned(
                &fetcher,
                &reference,
                include_bytes!("../../owned/fixtures/troy_725218.json"),
            )?;
            reference.meet_id = "+725218".to_string();
            match crate::milesplit::fetch_owned_meet(
                &fetcher,
                &reference,
                &crate::net::FetchOptions::default(),
            )
            .await
            {
                Err(crate::CrawlError::Schema { detail, .. }) => {
                    check!(eq; detail, "invalid structured meet ID");
                }
                other => {
                    return Err(format!(
            "noncanonical source identity must not acquire another meet's capture: {other:?}"
        )
                    .into())
                }
            }
            check!(eq; fetcher.stats().await.cache_hits, 0);
            Ok(())
        })
}
