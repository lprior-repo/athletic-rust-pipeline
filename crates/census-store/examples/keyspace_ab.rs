use fjall::{Database, Keyspace, KeyspaceCreateOptions, OwnedWriteBatch, PersistMode, Readable};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use tempfile::tempdir;

const BATCH_ROWS: u64 = 25_000;
const READ_SAMPLE: u64 = 1_000;

struct Args {
    evidence_rows: u64,
    derived_rows: u64,
    generations: u64,
    seed: u64,
}

#[derive(Clone, Copy)]
struct Shape<'a> {
    table: &'a str,
    second: u64,
    rows: u64,
    base: u64,
    span: u64,
}

impl Shape<'_> {
    fn seeded(table: &str, rows: u64, base: u64, span: u64) -> Shape<'_> {
        Shape {
            table,
            second: 0,
            rows,
            base,
            span,
        }
    }

    fn at(self, second: u64) -> Self {
        Self { second, ..self }
    }
}

fn fault(context: &str, error: impl std::fmt::Display) -> ExitCode {
    eprintln!("{context}: {error}");
    ExitCode::FAILURE
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        evidence_rows: 200_000,
        derived_rows: 400_000,
        generations: 4,
        seed: 1,
    };
    let mut rest = env::args().skip(1);
    while let Some(flag) = rest.next() {
        match flag.as_str() {
            "--evidence-rows" => args.evidence_rows = number(&mut rest, &flag)?,
            "--derived-rows" => args.derived_rows = number(&mut rest, &flag)?,
            "--generations" => args.generations = number(&mut rest, &flag)?,
            "--seed" => args.seed = number(&mut rest, &flag)?,
            other => return Err(format!("unknown arg: {other}")),
        }
    }
    Ok(args)
}

fn number(rest: &mut impl Iterator<Item = String>, flag: &str) -> Result<u64, String> {
    rest.next()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| format!("bad value for {flag}"))
}

fn lcg(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    }
}

fn body(rng: &mut impl FnMut() -> u64, base: u64, span: u64) -> Vec<u8> {
    let length = rng().checked_rem(span).unwrap_or(0).saturating_add(base);
    let mut value = Vec::new();
    for _ in 0..length {
        let code = rng().checked_rem(90).unwrap_or(0).saturating_add(33);
        value.push(u8::try_from(code).unwrap_or(b'?'));
    }
    value
}

fn keyed(table: &str, first: u64, second: u64) -> Vec<u8> {
    let mut key = table.as_bytes().to_vec();
    key.push(0);
    key.extend_from_slice(&first.to_be_bytes());
    key.push(0);
    key.extend_from_slice(&second.to_be_bytes());
    key
}

fn open(path: &Path, names: &[&str]) -> Result<(Database, Vec<Keyspace>), ExitCode> {
    let database = Database::builder(path)
        .open()
        .map_err(|error| fault("open db error", error))?;
    let mut spaces = Vec::new();
    for name in names {
        let space = database
            .keyspace(name, KeyspaceCreateOptions::default)
            .map_err(|error| fault("open keyspace error", error))?;
        spaces.push(space);
    }
    Ok((database, spaces))
}

fn persist(database: &Database) -> Result<(), ExitCode> {
    database
        .persist(PersistMode::SyncAll)
        .map_err(|error| fault("persist error", error))
}

fn flush(
    database: &Database,
    batch: OwnedWriteBatch,
    index: u64,
    rows: u64,
) -> Result<OwnedWriteBatch, ExitCode> {
    let done = index.saturating_add(1);
    if done.checked_rem(BATCH_ROWS).unwrap_or(1) != 0 && done < rows {
        return Ok(batch);
    }
    batch
        .commit()
        .map_err(|error| fault("batch commit error", error))?;
    Ok(database.batch())
}

fn stream_rows(
    database: &Database,
    space: &Keyspace,
    rng: &mut impl FnMut() -> u64,
    shape: Shape<'_>,
    writing: bool,
) -> Result<(Duration, u64), ExitCode> {
    let start = Instant::now();
    let mut bytes = 0_u64;
    let mut batch = database.batch();
    for index in 0..shape.rows {
        let held = if writing {
            let key = keyed(shape.table, index, shape.second);
            let value = body(rng, shape.base, shape.span);
            let held = key.len().saturating_add(value.len());
            batch.insert(space, key, value);
            held
        } else {
            batch.remove(space, keyed(shape.table, shape.second, index));
            0
        };
        bytes = bytes.saturating_add(u64::try_from(held).unwrap_or(u64::MAX));
        batch = flush(database, batch, index, shape.rows)?;
    }
    persist(database)?;
    Ok((start.elapsed(), bytes))
}

fn point_reads(
    database: &Database,
    space: &Keyspace,
    rng: &mut impl FnMut() -> u64,
    shape: Shape<'_>,
) -> Result<Duration, ExitCode> {
    let snapshot = database.snapshot();
    let start = Instant::now();
    for _ in 0..READ_SAMPLE {
        let id = rng().checked_rem(shape.rows).unwrap_or(0);
        if let Err(error) = snapshot.get(space, keyed(shape.table, shape.second, id)) {
            return Err(fault("read error", error));
        }
    }
    let elapsed = start.elapsed();
    drop(snapshot);
    Ok(elapsed)
}

fn tree_bytes(path: &Path) -> u64 {
    let mut total = 0_u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let held = if metadata.is_file() {
                metadata.len()
            } else {
                tree_bytes(&entry.path())
            };
            total = total.saturating_add(held);
        }
    }
    total
}

fn peak_rss_kb() -> Option<u64> {
    let text = fs::read_to_string("/proc/self/status").ok()?;
    let line = text.lines().find(|line| line.starts_with("VmHWM:"))?;
    line.split_whitespace().nth(1)?.parse::<u64>().ok()
}

fn arm_metrics(
    name: &str,
    database: &Database,
    evidence: &Keyspace,
    derived: &Keyspace,
    rng: &mut impl FnMut() -> u64,
    args: &Args,
    path: &Path,
) -> Result<(), ExitCode> {
    let evidence_shape = Shape::seeded("schools", args.evidence_rows, 120, 80);
    let derived_shape = Shape::seeded("source_identities", args.derived_rows, 90, 20);
    let insert = stream_rows(database, evidence, rng, evidence_shape, true)?;
    let mut write = (Duration::ZERO, 0_u64);
    let mut remove = Duration::ZERO;
    for generation in 1..=args.generations {
        let (duration, bytes) =
            stream_rows(database, derived, rng, derived_shape.at(generation), true)?;
        write = (
            write.0.saturating_add(duration),
            write.1.saturating_add(bytes),
        );
        if generation > 1 {
            let previous = derived_shape.at(generation.saturating_sub(1));
            remove = remove.saturating_add(stream_rows(database, derived, rng, previous, false)?.0);
        }
    }
    let reads = (
        point_reads(database, evidence, rng, evidence_shape)?,
        point_reads(database, derived, rng, derived_shape.at(args.generations))?,
    );
    println!(
        "arm {name}: insert={:?} write={:?} remove={:?} reads={:?} tree={} rss_kb={:?}",
        insert,
        write,
        remove,
        reads,
        tree_bytes(path),
        peak_rss_kb()
    );
    Ok(())
}

fn run_arm(name: &str, split: bool, args: &Args) -> Result<(), ExitCode> {
    let directory = tempdir().map_err(|error| fault("tempdir error", error))?;
    let layout: &[&str] = if split {
        &["evidence", "derived"]
    } else {
        &["entities"]
    };
    let (database, mut spaces) = open(directory.path(), layout)?;
    let derived = spaces.pop().ok_or(ExitCode::FAILURE)?;
    let evidence = if split {
        spaces.pop().ok_or(ExitCode::FAILURE)?
    } else {
        derived.clone()
    };
    let mut rng = lcg(args.seed);
    arm_metrics(
        name,
        &database,
        &evidence,
        &derived,
        &mut rng,
        args,
        directory.path(),
    )
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("usage: keyspace_ab [--evidence-rows N] [--derived-rows N] [--generations N] [--seed N]");
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    println!("keyspace-ab benchmark evidence_rows={}", args.evidence_rows);
    println!("derived_rows={}", args.derived_rows);
    println!("generations={}", args.generations);
    println!("seed={}", args.seed);
    for (name, split) in [("A", false), ("B", true)] {
        if let Err(code) = run_arm(name, split, &args) {
            return code;
        }
    }
    println!("verdict=comparison complete; inspect per-arm metrics above");
    ExitCode::SUCCESS
}
