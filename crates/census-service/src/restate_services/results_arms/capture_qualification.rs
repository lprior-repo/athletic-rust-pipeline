use std::sync::Arc;
use std::time::Instant;

use census_domain::UsJurisdiction;
use census_store::Store;
use sha2::{Digest, Sha256};

use super::{qualification_select, scope, Cache};

#[path = "capture_qualification/config.rs"]
mod config;
#[path = "capture_qualification/corpus.rs"]
mod corpus;
#[path = "capture_qualification/parity.rs"]
mod parity;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Default)]
struct Counts {
    capture_rows: u64,
    capture_keys: u64,
    partition_visits: u64,
    admitted: u64,
    pending: u64,
}

#[test]
fn synthetic_source_capture_profile() -> TestResult {
    let (profile, (directory, store)) = config::admit(config::Profile::read, |profile| {
        println!("source_capture_profile={} physical_rows={} payload_bytes={} jurisdictions=49 append_batch_rows={} input_preflight_bytes={} tmp_parent={:?}",
            profile.name, profile.rows, profile.payload.len(), config::BATCH_ROWS, profile.input_preflight, std::env::temp_dir());
        let directory = tempfile::tempdir()?;
        let store = Arc::new(Store::open(directory.path())?);
        Ok((directory, store))
    })?;
    let setup = Instant::now();
    corpus::seed(&store, &profile)?;
    let setup_elapsed = setup.elapsed();
    let mut cache = Cache::default();
    let mut held = Vec::new();
    held.try_reserve_exact(UsJurisdiction::CENSUS_SCOPE.len())?;
    let started = Instant::now();
    let mut counts = Counts::default();
    let mut digest = Sha256::new();
    UsJurisdiction::CENSUS_SCOPE
        .into_iter()
        .enumerate()
        .try_for_each(|(owner, jurisdiction)| {
            let (selected, measurement) =
                qualification_select(&mut cache, &store, scope(jurisdiction)?)?;
            check!(eq; measurement.partition_visits, parity::partition_rows(&profile, owner)?);
            if owner == 0 {
                check!(eq; measurement.capture.decoded_rows, profile.rows);
                check!(eq; measurement.capture.key_visits, profile.rows);
            } else {
                check!(eq; measurement.capture.decoded_rows, 0);
                check!(eq; measurement.capture.key_visits, 0);
            }
            parity::selected(&selected, &profile, jurisdiction, owner, &mut digest)?;
            counts.add(&selected, &measurement)?;
            println!(
                "source_capture_partition={} measurement={measurement:?}",
                jurisdiction.code()
            );
            held.push(selected);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;
    check!(eq; counts.capture_rows, profile.rows);
    check!(eq; counts.capture_keys, profile.rows);
    check!(eq; counts.partition_visits, profile.rows);
    check!(eq; held.len(), 49);
    println!("source_capture_complete profile={} physical_rows={} payload_bytes={} capture_rows={} capture_keys={} partition_visits={} admitted={} pending={} jurisdictions={} selected_sha256={:x} setup_elapsed={setup_elapsed:?} capture_selection_elapsed={:?}",
        profile.name, profile.rows, profile.payload.len(), counts.capture_rows, counts.capture_keys, counts.partition_visits,
        counts.admitted, counts.pending, held.len(), digest.finalize(), started.elapsed());
    drop(held);
    drop(cache);
    drop(store);
    directory.close()?;
    Ok(())
}

impl Counts {
    fn add(
        &mut self,
        selected: &crate::restate_services::results_arms::selection::Selected,
        measurement: &crate::restate_services::results_arms::capture::Measurement,
    ) -> TestResult {
        self.capture_rows = self
            .capture_rows
            .checked_add(measurement.capture.decoded_rows)
            .ok_or("capture rows overflow")?;
        self.capture_keys = self
            .capture_keys
            .checked_add(measurement.capture.key_visits)
            .ok_or("capture keys overflow")?;
        self.partition_visits = self
            .partition_visits
            .checked_add(measurement.partition_visits)
            .ok_or("visits overflow")?;
        self.admitted = self
            .admitted
            .checked_add(u64::try_from(selected.meets.len())?)
            .ok_or("admitted overflow")?;
        self.pending = self
            .pending
            .checked_add(u64::try_from(selected.pending.len())?)
            .ok_or("pending overflow")?;
        Ok(())
    }
}
