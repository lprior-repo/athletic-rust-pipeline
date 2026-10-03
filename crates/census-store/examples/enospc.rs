use census_store::{Application, Receipt, Store, StoreError, Table};
use std::error::Error;
use std::io;
use std::path::PathBuf;

#[path = "enospc/records.rs"]
mod records;
#[path = "enospc/reserve.rs"]
mod reserve;
#[path = "enospc/safety.rs"]
mod safety;
#[path = "enospc/verify.rs"]
mod verify;

const BASELINE_COUNT: usize = 4;
const BATCH_COUNT: usize = 32;
const MAX_ATTEMPTS: usize = 128;
const NOTE_BYTES: usize = 64 * 1024;
const RECOVERY_START: usize = BASELINE_COUNT + MAX_ATTEMPTS * BATCH_COUNT;
const MAX_RECORDS: usize = RECOVERY_START + BATCH_COUNT;

type ExampleResult<T> = Result<T, Box<dyn Error>>;

struct Acknowledged {
    start: usize,
    count: usize,
    receipt: Receipt,
}

struct Failed {
    start: usize,
    operation: String,
}

enum Attempt {
    Written(Acknowledged),
    Full(Failed),
}

fn commit_operation(
    store: &Store,
    operation: String,
    start: usize,
    count: usize,
) -> ExampleResult<Attempt> {
    let schools = records::schools(start, count)?;
    let digest = records::digest(&schools)?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &schools)?;
    match batch.commit_once(&operation, &digest) {
        Ok(Application::Written(receipt)) => {
            if receipt.operation != operation
                || receipt.digest != digest
                || receipt.appended != u64::try_from(count)?
                || receipt.at.is_empty()
            {
                return Err(
                    io::Error::other("written receipt does not match the exact batch").into(),
                );
            }
            Ok(Attempt::Written(Acknowledged {
                start,
                count,
                receipt,
            }))
        }
        Ok(Application::Repeated(_)) => {
            Err(io::Error::other("fresh operation replayed a preexisting receipt").into())
        }
        Err(error) if matches!(&error, StoreError::Write { .. }) && safety::is_enospc(&error) => {
            eprintln!("ENOSPC: atomic commit refused {operation} at school {start}: {error}");
            Ok(Attempt::Full(Failed { start, operation }))
        }
        Err(error) => Err(Box::new(error)),
    }
}

fn require_written(attempt: Attempt) -> ExampleResult<Acknowledged> {
    match attempt {
        Attempt::Written(ack) => Ok(ack),
        Attempt::Full(failed) => Err(io::Error::other(format!(
            "unexpected ENOSPC for required successful operation {}",
            failed.operation
        ))
        .into()),
    }
}

fn drain(store: &Store, acknowledged: &mut Vec<Acknowledged>) -> ExampleResult<Failed> {
    let failure = (0..MAX_ATTEMPTS).find_map(|attempt| {
        let start = attempt
            .checked_mul(BATCH_COUNT)
            .and_then(|offset| BASELINE_COUNT.checked_add(offset))
            .ok_or_else(|| io::Error::other("drain record index overflow"));
        let result = start.map_err(Box::<dyn Error>::from).and_then(|start| {
            commit_operation(store, format!("batch_{attempt}"), start, BATCH_COUNT)
        });
        match result {
            Ok(Attempt::Written(ack)) => {
                acknowledged.push(ack);
                None
            }
            Ok(Attempt::Full(failed)) => Some(Ok(failed)),
            Err(error) => Some(Err(error)),
        }
    });
    failure.ok_or_else(|| {
        io::Error::other(format!(
            "no kernel ENOSPC within {MAX_ATTEMPTS} bounded atomic batches"
        ))
    })?
}

fn replay(store: &Store, acknowledged: &[Acknowledged]) -> ExampleResult<()> {
    let baseline = acknowledged
        .first()
        .ok_or_else(|| io::Error::other("no acknowledged operation to replay"))?;
    let schools = records::schools(baseline.start, baseline.count)?;
    let digest = records::digest(&schools)?;
    if digest != baseline.receipt.digest {
        return Err(io::Error::other("replay supporting payload digest changed").into());
    }
    let before_walk = store.walk_table(Table::Schools)?;
    let before_sequence = store.snapshot().sequence();
    let before_receipts = store.receipt_count()?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &schools)?;
    let application = batch.commit_once(&baseline.receipt.operation, &digest)?;
    if application.appended() != 0
        || !matches!(&application, Application::Repeated(receipt) if receipt == &baseline.receipt)
    {
        return Err(io::Error::other(
            "acknowledged replay did not return the exact original receipt",
        )
        .into());
    }
    if store.walk_table(Table::Schools)? != before_walk
        || store.snapshot().sequence() != before_sequence
        || store.receipt_count()? != before_receipts
    {
        return Err(
            io::Error::other("acknowledged replay appended or mutated durable state").into(),
        );
    }
    Ok(())
}

fn main() -> ExampleResult<()> {
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(
        arguments
            .next()
            .ok_or_else(|| io::Error::other("usage: enospc <tmpfs-root>"))?,
    );
    if arguments.next().is_some() {
        return Err(io::Error::other("usage: enospc <tmpfs-root>").into());
    }
    let root = safety::inspect(&root)?;
    let reservation = reserve::Reservation::create(&root)?;
    let store = Store::open(&root)?;
    let mut acknowledged = Vec::new();
    acknowledged.try_reserve_exact(MAX_ATTEMPTS.saturating_add(2))?;
    acknowledged.push(require_written(commit_operation(
        &store,
        "baseline".to_string(),
        0,
        BASELINE_COUNT,
    )?)?);
    let failed = drain(&store, &mut acknowledged)?;
    drop(store);
    reservation.release()?;

    let store = Store::open(&root)?;
    verify::state(&store, &acknowledged, &failed)?;
    replay(&store, &acknowledged)?;
    verify::state(&store, &acknowledged, &failed)?;
    if store.receipt("recovery")?.is_some() {
        return Err(io::Error::other("new recovery operation already has a receipt").into());
    }
    acknowledged.push(require_written(commit_operation(
        &store,
        "recovery".to_string(),
        RECOVERY_START,
        BATCH_COUNT,
    )?)?);
    drop(store);

    let store = Store::open(&root)?;
    verify::state(&store, &acknowledged, &failed)?;
    println!(
        "PASS: kernel ENOSPC refused {}; {} exact acknowledged receipts survived; \
         replay appended zero; new atomic recovery batch survived cold reopen",
        failed.operation,
        acknowledged.len()
    );
    Ok(())
}
