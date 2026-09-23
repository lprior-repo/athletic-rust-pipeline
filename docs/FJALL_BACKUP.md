# Fjall backup / restore drill (objective §60)

Objective §60 asks for commands covering **consistent backup → restore → integrity verification → reopen
→ full census read**, and for an actual restore drill to be run before delivery. This document is that
drill: every command line below was executed on this machine against a real store built from the
pre-Fjall logs in this checkout (`var/census-service/`, gitignored runtime state - substitute your own
root), and the output blocks are verbatim.

The drill is enforced by [`crates/census-service/tests/backup_restore.rs`](../crates/census-service/tests/backup_restore.rs)
(7 tests, offline, no network, no shared state), so it is re-run by `cargo test -p census-service
--test backup_restore` rather than by hand. The shell sequence below exists so an operator can do the
same thing against a production root (a `<store-dir>` on disk, not a `/tmp` drill).

Schema, keyspaces and key format: [`FJALL_SCHEMA.md`](FJALL_SCHEMA.md). Store code: `src/store/`.

---

## 1. What a census store root contains

`<store-dir>/` as created by the CLI (`crates/census-service/src/store/mod.rs`):

| Path | Role | Needed in a backup? |
|---|---|---|
| `fjall/` | All canonical state: keyspaces `entities` (schools, teams, coaches, athletes, meets, events, performances), `journal` (per-phase dispatch logs), `meta` (markers, e.g. the legacy-import marker) | **Required** |
| `http/` | Source response cache | Optional - dropping it costs re-fetches, not correctness |
| `out/` | Regenerable exports (`report.json`, `census-by-state.csv`, `*.jsonl` snapshots) | No - rebuild with `consolidate` / `report` |
| `entities/`, `journal/` | Pre-Fjall JSONL logs; imported once at open and recorded by a marker in `meta` | No, once that marker exists |

`fjall/` in the drill store is 12 files / 229,232 bytes:

```text
$ (cd /tmp/census-backup-drill/live/fjall && find . -type f | sort)
./0.jnl
./keyspaces/0/current
./keyspaces/0/tables/3
./keyspaces/0/v5
./keyspaces/1/current
./keyspaces/1/v0
./keyspaces/2/current
./keyspaces/2/v0
./keyspaces/3/current
./keyspaces/3/v0
./lock
./version
```

`lock` is the advisory lock file, `0.jnl` the write-ahead journal (223,557 of those bytes) and `version`
a format marker. `keyspaces/<n>/` are fjall's index-addressed keyspace directories: index `0` is fjall's
own keyspace-configuration table - the single SST in `keyspaces/0/tables/3` is where the census keyspace
names are recorded, in creation order (`entities`, `journal`, `meta`) - so the census's three keyspaces
live in `1`-`3`, journal-only in this store. The set grows as compaction
writes SSTs, so **copying a live directory by an include-list of "files we have seen before" is wrong**;
copy the whole `fjall/` subtree.

## 2. What the store API can and cannot do

Read out of the pinned dependency (`fjall-3.1.10`), not assumed:

* **No consistent-backup entry point exists in the census API.** `Store` keeps `db: Database` private
  and exposes no snapshot. fjall does provide `Database::snapshot()` (`fjall-3.1.10/src/db.rs:150`)
  returning a `Snapshot` (`src/snapshot.rs:17-31`) that pins the LSM version for as long as it is
  held - the thing a copier needs so compaction cannot delete a file mid-copy - but the census crate
  never calls it and gives a caller no way to hold one.
* **There is no pause point during a backup.** The only quiescence primitives are process-level: the
  handle's lock and `Store::flush()` (= `Database::persist(PersistMode::SyncAll)`,
  `fjall-3.1.10/src/db.rs:350`), which fsyncs but does **not** stop writers.
* **Cold copy is therefore enforced, not merely conventional.** fjall takes the lock through
  `LockedFileGuard::try_acquire` on `fjall/lock`
  (`fjall-3.1.10/src/locked_file.rs:49-80`, `LOCK_FILE = "lock"` at `src/file.rs:11`), which calls
  `std::fs::File::try_lock` - `flock(2)` on Linux. It retries three times 100 ms apart and then returns
  `fjall::Error::Locked`, so a second opener fails in about a quarter of a second rather than blocking.
  Measured with `flock(1)` holding the same file a running unit holds:

  ```console
  $ flock -n /tmp/census-backup-drill/live/fjall/lock -c 'sleep 3' &
  $ sleep 0.5
  $ target/debug/census-service --store /tmp/census-backup-drill/live fjall-stats
  Error: store open failed: FjallError: Locked

  Caused by:
      FjallError: Locked
  $ echo $?
  1
  ```

  The same lock covers the legacy import: `Store::open` imports pre-Fjall `entities/`/`journal/` logs
  before handing the store back (`crates/census-service/src/store/mod.rs:261` →
  `store/legacy.rs:21-45`), under the same locked handle, and the CLI's `import-legacy` command
  (`src/cli/store.rs:25-42`) merely reports on the already-open store. So an import can never race a
  root another process is serving - but it does mean **opening a copy that still carries legacy logs
  writes into that copy** (markers, and the imported rows). On the source root nothing is re-imported,
  because the markers travel inside `fjall/`'s `meta` keyspace.
* **A copy that races a live writer loses the incomplete tail silently.** Recovery truncates a torn
  journal tail instead of failing the open (`fjall-3.1.10/src/journal/reader.rs:56-79`), so a snapshot
  copy of `fjall/` is simply *missing* the rows whose batch frame was still being written when the
  copy reached it. No error, no warning: the census just reads back with lower counts. The suite
  measures this (`a_copy_whose_journal_stops_mid_batch_keeps_the_complete_prefix`, §4).

Not filed as a bug against the census crate; recorded here so nobody ships a "hot backup" script
believing the store cooperates. The two shapes that would make a hot backup safe are exposing
`fjall::Snapshot` from `Store` (hold the snapshot, copy `fjall/`, drop it) or a `Store::backup_to(dir)`.
Neither exists today, so the rest of this document is **cold backup only**.

## 3. The drill, command by command

Store built from this checkout's raw logs (`var/census-service/`) so the drill carries real data
without touching the network: 200 schools + 100 coaches + 100 meets = **400 observations**, plus one
journal phase.

```console
$ DRILL=/tmp/census-backup-drill
$ mkdir -p $DRILL/live/entities $DRILL/live/journal
$ head -n 200 var/census-service/entities/schools.jsonl > $DRILL/live/entities/schools.jsonl
$ head -n 100 var/census-service/entities/coaches.jsonl > $DRILL/live/entities/coaches.jsonl
$ head -n 100 var/census-service/entities/meets.jsonl   > $DRILL/live/entities/meets.jsonl
$ head -n 1   var/census-service/journal/kshsaa_schools.jsonl > $DRILL/live/journal/kshsaa_schools.jsonl
$ target/debug/census-service --store $DRILL/live import-legacy
legacy	schools	89426 bytes	/tmp/census-backup-drill/live/entities/schools.jsonl
legacy	teams	absent
legacy	coaches	49163 bytes	/tmp/census-backup-drill/live/entities/coaches.jsonl
legacy	athletes	absent
legacy	meets	61487 bytes	/tmp/census-backup-drill/live/entities/meets.jsonl
legacy	events	absent
legacy	performances	absent
legacy_journal_dir	/tmp/census-backup-drill/live/journal
store	/tmp/census-backup-drill/live
schools	200
teams	0
coaches	100
athletes	0
meets	100
events	0
performances	0
observations	400
bytes_on_disk	0
```

### 3.1 Consistent backup - stop the unit, then copy

```console
$ target/debug/census-service --store $DRILL/live fjall-stats
store	/tmp/census-backup-drill/live
schools	200
teams	0
coaches	100
athletes	0
meets	100
events	0
performances	0
observations	400
bytes_on_disk	0
store_bytes	429419
$ cp -a $DRILL/live $DRILL/backup
$ (cd $DRILL/live/fjall     && find . -type f | sort | xargs sha256sum) > $DRILL/live.manifest
$ (cd $DRILL/backup/fjall   && find . -type f | sort | xargs sha256sum) > $DRILL/backup.manifest
$ diff $DRILL/live.manifest $DRILL/backup.manifest && echo "MANIFESTS IDENTICAL ($(wc -l < $DRILL/live.manifest) files)"
MANIFESTS IDENTICAL (12 files)
```

`cp -a <store-dir> <backup-dir>` copies `fjall/` along with the optional `http/` cache; `cp -a
<store-dir>/fjall <backup-dir>/fjall` is enough for a state-only backup. Either form creates the
destination directory, so give it a path that does not exist yet - `cp -a` nests into an existing
directory (`cp -a a/fjall b/fjall` with `b/fjall` present produces `b/fjall/fjall`), which a restore
would then not find. **The unit must not be running**: while it is, the copy is the racing case of §2.

Three sizes are in play and they answer different questions; the numbers below are this drill's store:

| `fjall-stats` line | live root | restored root | What it counts |
|---|---|---|---|
| `bytes_on_disk` | 0 | 0 | fjall's LSM-tree level sizes - SST files only (`Keyspace::disk_space`, `fjall-3.1.10/src/keyspace/mod.rs:401` → `lsm-tree-3.1.10/src/tree/mod.rs:615-620`). A store whose rows still live in the journal reports 0, and reads 0 again for every damaged copy in §3.5 |
| `store_bytes` | 429,419 | 229,232 | the recursive size of the store root (`store/read.rs:250-262`, reported at `store/read.rs:127`): database, journal, HTTP cache, `out/` and the pre-Fjall logs. `du -sb` agrees |

So `bytes_on_disk 0` is not a bug and not a restore check either: it is 0 before the damage and 0
after it. Size a copy by `store_bytes` (or `du -sb`), and prove a restore by the journal manifest, the
census document and `Store::stats()`, never by a size column.

**Sharp edge, measured.** fjall preallocates a fresh journal to 64 MiB
(`fjall-3.1.10/src/journal/writer.rs:19`, `set_len(64 * 1024 * 1024)`) and only a later *reopen*
truncates it back to the frames actually written. On this machine, one `import-legacy` run over 200
schools left a store of **67,212,350** bytes on disk (`0.jnl` = 67,108,864 bytes, and `du` counted all
of it - the preallocation is not sparse on this filesystem); one `fjall-stats` run later the same store
was **196,402** bytes with a 101,301-byte journal:

```console
$ target/debug/census-service --store $D import-legacy | tail -3
observations	200
bytes_on_disk	0
store_bytes	67212350
$ ls -la $D/fjall/0.jnl
-rw-r--r-- 1 lewis lewis 67108864 ... /tmp/census-backup-drill2/fjall/0.jnl
$ du -sb $D/fjall
67116338	/tmp/census-backup-drill2/fjall
$ target/debug/census-service --store $D fjall-stats | tail -2
bytes_on_disk	0
store_bytes	196402
$ ls -la $D/fjall/0.jnl
-rw-r--r-- 1 lewis lewis 101301 ... /tmp/census-backup-drill2/fjall/0.jnl
$ du -sb $D/fjall
106976	/tmp/census-backup-drill2/fjall
```

A backup taken right after a collection run therefore carries up to 64 MiB of journal for a few hundred
rows. Reopening the store once (any command) settles it and costs nothing, so stop the unit, run one
`fjall-stats`, and only then copy.

### 3.2 Restore into a fresh data directory

```console
$ mkdir -p $DRILL/restored && cp -a $DRILL/backup/fjall $DRILL/restored/fjall
$ (cd $DRILL/restored/fjall && find . -type f | sort | xargs sha256sum) > $DRILL/restored.manifest
$ diff $DRILL/backup.manifest $DRILL/restored.manifest && echo "RESTORED MANIFEST IDENTICAL"
RESTORED MANIFEST IDENTICAL
```

`<store-dir>/fjall` is the whole restore input: the CLI takes `--store <dir>`, creates the directory if
missing, and has no restore subcommand - restoring is copying `fjall/` onto a fresh root. The fresh root
must **not** contain the pre-Fjall `entities/`/`journal/` logs of the source root; if it does, the
legacy import is marker-guarded per table (`imported:<table>` in the `meta` keyspace travels with the
database, `crates/census-service/src/store/legacy.rs:12-45`), so the logs are not read a second time -
and if the marker were ever missing, a re-import appends at a freshly reserved sequence base instead of
overwriting (`legacy.rs:48-91`). Do not restore a backup over a root that is
being served - the lock (§2) will reject it, or worse, a running process holds an in-memory view of the
files you are replacing.

### 3.3 Integrity verification

The manifest diff above is the integrity check that matters for a backup: every file name and every
SHA-256 of the 12-file tree is identical between source, backup, and restored copy. On the database
side, reopening runs fjall's own recovery - `version` and per-keyspace `current` descriptors are read
back and the journal is replayed; a damaged descriptor fails the open rather than limping.

### 3.4 Reopen and full census read

```console
$ target/debug/census-service --store $DRILL/restored fjall-stats
store	/tmp/census-backup-drill/restored
schools	200
teams	0
coaches	100
athletes	0
meets	100
events	0
performances	0
observations	400
bytes_on_disk	0
store_bytes	229232
$ target/debug/census-service --store $DRILL/restored consolidate
schools	199
teams	0
coaches	100
coaches_email_withheld	0
athletes	0
meets	100
events	0
performances	0
$ target/debug/census-service --store $DRILL/restored report
wrote /tmp/census-backup-drill/restored/out/report.json
wrote /tmp/census-backup-drill/restored/out/census-by-state.csv
scope=all_sources totals: schools=199 athletes=0 co2027=0 (boys=0 girls=0) profile_url=0 multisource=0 coaches=100
$ target/debug/census-service --store $DRILL/live report
wrote /tmp/census-backup-drill/live/out/report.json
wrote /tmp/census-backup-drill/live/out/census-by-state.csv
scope=all_sources totals: schools=199 athletes=0 co2027=0 (boys=0 girls=0) profile_url=0 multisource=0 coaches=100
$ jq -S 'del(.store_dir,.generated_on)|.notes|=map(sub("from .*";"from <root>"))' $DRILL/live/out/report.json     > $DRILL/live-norm.json
$ jq -S 'del(.store_dir,.generated_on)|.notes|=map(sub("from .*";"from <root>"))' $DRILL/restored/out/report.json > $DRILL/restored-norm.json
$ wc -c $DRILL/live-norm.json $DRILL/restored-norm.json && diff $DRILL/live-norm.json $DRILL/restored-norm.json; echo "diff exit=$?"
27649 /tmp/census-backup-drill/live-norm.json
27649 /tmp/census-backup-drill/restored-norm.json
55298 total
diff exit=0
$ jq -c '{scope, totals: {schools: .totals.schools, coaches: .totals.coaches, coaches_with_email: .totals.coaches_with_email}, meets: .meets.total}' $DRILL/restored/out/report.json
{"scope":"all_sources","totals":{"schools":199,"coaches":100,"coaches_with_email":25},"meets":100}
```

Every count matches the source store: `fjall-stats` shows the same 200/100/100/400 before and after,
`consolidate` and `report` produce the same reduction on both roots, and after removing only the two
fields that are *supposed* to differ (the root path and the run date) the two census documents are
byte-identical (`diff` exit 0, 27,649 bytes each; the earlier drill measured 4,519 bytes on the
same corpus - the document grows as the report module lands coverage, so quote the *equality*, not the
size).

`jq` on this machine is `jaq 2.3.0` at `/usr/bin/jq`; it rejects non-JSON prefixes, so do not pipe
`report --print` (whose stdout starts with three `wrote ...` lines) into it - use `out/report.json`.

### 3.5 What damage does, measured

The verification above is a *file* check (§3.1) plus a *content* check (§3.4); neither is redundant, and
the shape of a damaged restore is worth knowing before trusting either one:

| Damage | Result |
|---|---|
| `fjall/version` replaced with garbage | `Error: store open failed: FjallError: InvalidVersion(None)`, exit 1 |
| `fjall/keyspaces/0/current` truncated to zero bytes | `Error: store open failed: FjallError: Storage(Io(Error { kind: UnexpectedEof, message: "failed to fill whole buffer" }))`, exit 1 |
| `fjall/0.jnl` truncated by 100 bytes | opens, exit 0; `fjall-stats` still reports 200/100/100/400 (`bytes_on_disk 0`, `store_bytes 458251`) and the census document is byte-identical to the intact restore - the lost tail frame carried nothing the census reads |
| `fjall/0.jnl` truncated by 4,000 bytes | opens, exit 0; `fjall-stats` now reports `schools 200 coaches 100 meets 0 observations 300` (`store_bytes 391138`), and the census loses the whole timer-meet breakdown |
| `fjall/0.jnl` with one byte flipped inside a stored row value | **every one of the 14 value-byte tears refused the open** (`cargo test … backup_restore -- --nocapture`: `row-tear sweep over 14 bytes: 14 refused the open, 0 opened with every row intact`). A tear in the *value* is detected; a tear in the *header* can panic inside fjall itself (next row) |
| `fjall/0.jnl` with one byte flipped inside fjall's entry header | a probe over the 96 header bytes of one frame: 8 panicked, 82 refused the open, 6 opened. The panic is fjall's **own debug assertion**, not a census error - and in a debug build it takes the process down: |

```console
$ rm -rf $DRILL/dmg-a && cp -a $DRILL/restored $DRILL/dmg-a && printf 'XXXX' > $DRILL/dmg-a/fjall/version
$ target/debug/census-service --store $DRILL/dmg-a fjall-stats
Error: store open failed: FjallError: InvalidVersion(None)

Caused by:
    FjallError: InvalidVersion(None)
$ echo $?
1
$ rm -rf $DRILL/dmg-c && cp -a $DRILL/restored $DRILL/dmg-c && truncate -s -100 $DRILL/dmg-c/fjall/0.jnl
$ target/debug/census-service --store $DRILL/dmg-c fjall-stats | tail -3
observations	400
bytes_on_disk	0
store_bytes	458251
$ target/debug/census-service --store $DRILL/dmg-c consolidate >/dev/null && target/debug/census-service --store $DRILL/dmg-c report >/dev/null
$ jq -S 'del(.store_dir,.generated_on)|.notes|=map(sub("from .*";"from <root>"))' $DRILL/dmg-c/out/report.json > $DRILL/dmg-c-norm.json
$ diff $DRILL/restored-norm.json $DRILL/dmg-c-norm.json; echo "diff exit=$?"
diff exit=0
# the header-tear probe of the last table row, verbatim (temporary test, removed after measuring):
thread 'temp_probe_entry_header_tear' (1784428) panicked at /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.10/src/journal/entry.rs:190:25:
assertion `left == right` failed
  left: 4278190245
 right: 165
```

A damaged **descriptor** closes the door; a damaged **journal** opens and answers, because recovery
truncates at the last complete frame (`journal/reader.rs:56-79`). So "it opened" proves nothing about a
restore, and neither size column does either: `bytes_on_disk` reads `0` on the intact restore *and* on
every damaged copy in the table, while `store_bytes` (the recursive root size, `store/read.rs:250-262`)
counts real bytes but says nothing about which rows survived - it even *grows* to 458,251 on the truncated
copy, because that copy carries the consolidated snapshots of the root it was taken from. `fjall-stats`
reports what survived the recovery, not what was backed up. Verify a restore by digest (§3.1) or by
reading the census and comparing it (§3.4) - which is exactly what the executable drill in §4 asserts,
at the level of both the read model and the database files.

One consequence for operators: a byte flipped in fjall's entry header is a **process panic in a debug
build**, which no `Result` in this crate can catch - the assertion is `debug_assert_eq!(value_len,
on_disk_value_len)` in the uncompressed arm of `Entry::decode_from`
(`fjall-3.1.10/src/journal/entry.rs:190`; uncompressed is the arm the census writes, because its rows
are small). In a release build that assertion is compiled out and the frame should be dropped like any
other corrupt frame; that release behaviour is **not measured here** (the whole drill ran against debug
binaries, which is what `cargo test` and the repo's own tooling build). Treat it as
expected-but-unverified, and prefer copying a *stopped* store over coping with a torn one.

## 4. The executable drill

```console
$ cargo test -p census-service --test backup_restore
```

The suite covers what the shell sequence cannot do deterministically: a copy taken from an open handle
with no flush, a journal cut mid-batch, a journal flipped mid-frame, the lock refusing a second opener,
and the restored copy's observation history row for row.

| Test | What it proves |
|---|---|
| `cold_copy_backup_restores_the_read_model_exactly` | the whole drill end to end. The corpus is asserted against the *live* store first (so a drill that loses the same rows on both sides still fails), then the restored copy must reproduce it surface for surface: per-table observation counts, observation total, merged-school count, the history school's merged observation history, the core-scope census JSON, the all-sources census JSON, the best-mark reduction, the consolidated row counts and the consolidated snapshots. The backup and the restore are each asserted to be a byte image of the previous stage (`tree_digest` over `fjall/`) |
| `the_restored_store_reopens_and_appends_without_overwriting` | a restore that seeded its sequence counter wrong would hand out a sequence already in use and overwrite a restored observation. After the restore an append must add a row rather than replace one, join the history as a fourth observation, survive a reopen, and resume from the restored store's high-water mark |
| `a_copy_taken_while_the_store_handle_is_open_keeps_every_committed_batch` | a copy taken from an open handle with **no** `flush()` is still complete, because `append_many` commits at `SyncData`. Pins the regression that matters for backups: a durability downgrade to `Buffer` would make the copy lose its unpersisted tail - silently |
| `a_second_open_of_a_live_store_is_refused` | the "cold" in cold copy is enforced by the database, not by convention: a second `Store::open` of a live store fails with the typed `StoreError::Open`, and the path opens again once the first handle is dropped |
| `the_restored_store_does_not_re_import_the_legacy_journals_it_carries` | a whole-root backup carries `entities/*.jsonl` and `journal/*.jsonl` next to the database; the import markers in the copied `meta` keyspace must stop a restore from doubling the counts of exactly the tables that still have a log. Asserted over two consecutive opens, including the resume-journal keys |
| `a_copy_whose_journal_stops_mid_batch_keeps_the_complete_prefix` | the racing copy itself. Cutting one byte into the second batch's frame keeps **exactly** the first batch (6 rows → 2), every surviving row still merges, the lost batch's rows are gone rather than invented, and the source store is untouched by the copy's damage. Cutting one byte short of the journal keeps exactly the first two batches |
| `a_copy_with_a_torn_byte_inside_a_row_never_yields_an_invented_row` | the other half of that hazard: each byte of a real row *value* is flipped in turn (14 bytes) and every outcome must be one of "refused the open" or "opened with every surviving row byte-identical to the source". Measured: 14 refused, 0 opened - the tear is detected, never silently absorbed - and no row is ever changed or invented. The counts are printed, so run with `--nocapture` to see them. The tear is aimed at a value on purpose: one byte earlier, in fjall's entry header, is fjall's own debug assertion (§3.5) |

The suite is offline and self-contained: the corpus is synthetic and built in the test file itself
(three schools across Wisconsin, Minnesota and Iowa with their teams, coaches, meets and athletes, plus
one recorded resume unit of work), each test builds its store inside its own `tempfile::TempDir`, and
no request leaves the process.

## 5. Not covered by this drill

* **A backup of a large live store.** The drill's store never flush()es, so its rows live in the
  journal; a real campaign flushes SSTs and compacts. The racing-copy behaviour is therefore measured
  for *one* shape (torn journal) and not for the many-file shape (compaction rewriting SSTs while a
  copier reads them) - which is exactly the case a snapshot would protect, and exactly the case no
  census API can currently express.
* **Restoring while a unit is serving.** Not attempted: the lock forbids part of it, and nothing
  coordinates the in-memory view.
* **A whole-root restore at CLI level.** The shell drill restores `fjall/` only; the case where the
  pre-Fjall `entities/`/`journal/` logs travel with the restore is covered by
  `the_restored_store_does_not_re_import_the_legacy_journals_it_carries` (§4), not repeated here.
* **The `http/` response cache** is treated as optional and never verified after a restore.
* **Cross-machine and cross-filesystem restore** (permission bits, sparse files, encrypted volumes).
* **A supported hot backup.** Impossible without an API change; see §2.
* **A journal that passes its 64 MiB preallocation and rotates.** The drill's journal is 223 KB; fjall
  rotates the active journal to `<journal_id + 1>.jnl` when it fills (`journal/writer.rs:66`, `fn
  rotate`, which creates the next id via `create_new`), so a real
  campaign's backup input is several journal files, and the drill never crossed that boundary.
* **The release-build behaviour of a torn entry header.** §3.5 measures the debug-build outcome - a
  panic inside fjall's own `debug_assert_eq!` - and argues the assertion is compiled out in release;
  no release binary was built or run for this drill (the repo's tooling builds debug, and a release
  build would have serialised the shared target directory).
* **Pinning the header-tear panic in the suite.** Deliberate: the test would have to assert a panic
  that only exists because fjall's *debug* assertions are on, and a temporary probe (96 header bytes:
  8 panicked, 82 refused, 6 opened) was removed after measuring rather than committed as a test that
  a future fjall release may legitimately change.
* **Windows.** `std::fs::File::try_lock` (Rust's own file locking) does not map to `flock(2)` there, so
  the lock demonstration in §2 is Unix-only.
