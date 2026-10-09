# Fjall cold backup and restore

Canonical operator procedure. The store's own table and key definitions
(`crates/census-store/src/table.rs`, `crates/census-store/src/keys.rs`) own schema/read APIs;
the verification ledger (removed 2026-10-09) retains the historical small-store and
2026-09-24 campaign drill measurements. Those transcripts are not rerun or fresh-run certification.

## Safety and included state

Stop intake, drain and close the sole store-owning process before backup. The backup API takes the
Fjall lock and refuses a live owner. `Store::snapshot()` pins an in-process MVCC read view; it does
not make an arbitrary live-directory copy consistent across journal rotation and compaction.
`Store::snapshot()` also takes the store's append lock while it pins that view, so the view and both
generation counters (`evidence_generation`, `derived_generation`) describe one write boundary rather
than a batch committed between the two reads. A reader waits only for the commit critical section,
not for the reads that follow it.
`flush()` makes writes durable but does not pause writers. Cold backup is the supported path.

Current `Store::backup` copies existing `fjall/`, `entities/`, `journal/`, `http/` and `out/` subtrees,
opens the staged copy for count validation, writes `backup.json` with file lengths/SHA-256, link
targets and table counts, and publishes the staged generation. A symbolic link is copied when its
target is relative and stays inside the store: the published workbook pointer
`out/publication/current -> generations/<digest>` is such an object, and a backup that dropped or
followed it would not be a complete store. The manifest records that target and the SHA-256 of the
target string, and the backup report counts it under `links`. Absolute links, links that leave the
store, sockets and other nonregular objects are refused.
`Store::restore` checks the manifest/files and restored counts before promoting into its destination,
and recreates every recorded link. Use a new destination for a drill; never restore over a served root.
Restore rejects a symlink backup root or manifest, and checks every intermediate component and final
file named by the manifest before staging and again before copying. A link entry's final component is
the link itself: restore accepts it only when the link on disk carries exactly the recorded target,
that target is relative and stays inside the backup, and the recorded SHA-256 matches the target
string restore would create. These metadata checks reject a
static symlink escape, not a malicious concurrent path-replacement race: keep the cold backup tree
owned and immutable throughout restore.

Durability contract: both verbs fsync every copied file, then every directory of the staged
generation bottom-up (deepest first) before the publish rename, and the destination's parent
afterwards. An acknowledged backup or restore therefore survives a host crash on a filesystem where
directory fsync is honoured; a filesystem that acknowledges or silently discards directory fsync
(some network and overlay filesystems) is outside the supported contract, and the documented reboot
qualification is what establishes it for the filesystems actually used.

Native Restate's durable directory is **separate** and not included. Nor are arbitrary external raw
capture paths. Inventory every referenced evidence object and run/decision/artifact manifest before
calling a backup complete. If `http/` holds the only retained source bytes, it is evidence, not an
optional optimization that may be dropped on the assumption the source can be fetched again.
Historical JSONL logs are preserved where present; no current `import-legacy` CLI is implied.

## Procedure

Run only against a stopped, owned source store and fresh backup/restore destinations. Commands below
are templates; choose paths for the actual run and record them with build, schema and manifest IDs.

```sh
census-service --store <source-store> store-integrity
census-service --store <source-store> fjall-stats
census-service --store <source-store> store-backup --to <backup-dir>
census-service --store <fresh-unused-path> store-restore --from <backup-dir> --to <new-restored-store>
census-service --store <new-restored-store> store-integrity
census-service --store <new-restored-store> fjall-stats
census-service --store <new-restored-store> consolidate
census-service --store <new-restored-store> report
```

`store-restore` is dispatched through the CLI's shared store open, so its `--store` root must itself
open as a store; give it a fresh unused path (the drill script passes one under its own temporary
directory), never the destination it is about to create.

Inspect `store-integrity`'s `ok` and per-table findings, not just process success. Preserve the source
baseline and backup manifest. Compare the complete physical table map, effect receipts, source IDs,
canonical decisions/history and semantic records, not only aggregate row counts. Recompute required
PR/cohort/contact/coverage checks and independently verify the actual workbook against the restored
input generation. Equal totals can conceal changed ownership, missing evidence or wrong mark units.

Opening the restored database can rewrite engine files during recovery. Validate the manifest at the
restore boundary; do not demand byte identity of mutable database files after later opens/writes.
Journal preallocation means footprint may shrink on reopen without any evidence loss. Conversely,
a truncated journal may open with only a complete prefix: “it opened” is not integrity proof.

## Existing automated drill

```sh
env -u CI tools/moon-local run pipeline:build-portable
mkdir -p var/restore-drill-tmp
TMPDIR="$PWD/var/restore-drill-tmp" env -u CI tools/moon-local run pipeline:backup-drill -- <stopped-source-store>
tools/moon-local run pipeline:tests -- -E 'binary(backup_restore)'
```

The operator script creates owned scratch state, performs backup/restore/integrity/count checks,
consolidates and compares normalized report reads, then removes its scratch directory. The drill
defaults to the optimized portable Moon CLI built above. Use local disk with capacity for backup, restored database and materialized
outputs; do not point a destructive fault at the source. An empty-store pass proves little about a
nonempty census. These commands are procedures, not evidence that they ran during this docs cleanup.

The integration suite separately covers cold copies, reopens/appends, lock exclusion and damaged
journal boundaries. Its narrower assertions do not replace the full publication/readback oracle or
all [17 native fault scenarios](NATIONAL-CENSUS-FAULTS.md).

## Limits and failure handling

- A held lock means stop the owner; never remove/bypass the lock file.
- Corrupt manifest, descriptor, payload or missing external capture blocks acceptance. Preserve the
  failed generation and diagnostics without replacing the previous accepted backup.
- Interrupt backup/restore only in an isolated owned scenario; verify the original and prior accepted
  generation remain recoverable and partial staging cannot become the selected backup.
- Historical journal header-tear probes hit debug assertions inside Fjall. Their release-build
  behavior was not established by that probe; do not advertise panic-free recovery from it.
- Live compaction-copy safety, Restate journal restoration, host reboot, cross-machine/filesystem
  migration and Windows behavior require their own evidence. A Fjall drill does not prove them.
