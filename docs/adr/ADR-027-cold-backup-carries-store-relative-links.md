# ADR-027 - Cold backup carries the store's relative links; escapes stay refused

**Status:** Accepted (2026-10-07).

## Context

[ADR-001](ADR-001-fjall-primary-store.md) makes Fjall the sole evidence store, and
[ADR-026](ADR-026-derived-generations-and-store-schema.md) publishes derived state as a generation
directory plus one atomic pointer flip. That pointer is a symbolic link inside the store's own tree:
`out/publication/current -> generations/<generation-digest>` (the same relative-pointer convention
[ADR-020](ADR-020-school-address-corpus-port.md) uses for the address corpus). A published store
therefore contains a symlink by design — it is a store object, not an accident of the filesystem.

`Store::backup` refused every symbolic link, so a backup of a *published* store was impossible: it
stopped on the pointer it was supposed to preserve (`dtdq`; observed 2026-10-06 on
`var/assoc-tn-20261004`, whose S15 drill had passed minutes before the pointer was created). The
refusal was not an isolated check: `digest_tree` refused links by kind as well.

Restore's refusal of symlinks is a security property, not a copy policy (gap `06o`): a backup root,
manifest or intermediate component that is a symlink may redirect the copy outside the tree, so
`Store::restore` walks every component and refuses one. Any change has to keep that boundary.

## Decision

1. **The backup unit is every object the store owns.** A symbolic link is copied as a link when its
   target is relative and lexically stays inside the store root. The publication pointer is the
   motivating object; the rule is general, not a name exception for `current`.
2. **Escapes stay refused.** An absolute target, an empty target, or a target whose `..` components
   climb past the store root is refused with the link's path and target named; sockets, fifos, devices
   and other nonregular objects are refused as before. Containment is lexical component accounting,
   which matches the already-documented boundary: these checks reject a static symlink escape, not a
   malicious concurrent path-replacement race against an owned, immutable cold tree.
3. **The manifest records the link, not the target's bytes.** `ManifestEntry.target` carries the link
   text, `length` is `0`, and `sha256` is the digest of the target string. The manifest stays version
   `2`: the field is optional, so manifests written before this change still restore, and a reader
   that ignores it refuses the entry as a nonregular object rather than silently copying something
   else.
4. **Restore validates the link twice and then recreates it.** Validation checks that the declared
   target is relative and stays inside the backup, that the object on disk is a symlink carrying
   exactly that target, and that the recorded digest matches; only then is the link created in the
   staging tree, so a tampered manifest or a swapped link refuses the whole restore instead of
   publishing a redirected pointer. Restore never follows the link, so no bytes of the target enter
   the copy through it.
5. **Reports count links separately.** `BackupReport`/`RestoreReport` gain `links`; `files` and
   `bytes` continue to describe regular files only, so existing totals keep their meaning.

## Alternatives considered

- **Dereference and copy the target content.** Rejected: it duplicates the generation's bytes, loses
  the pointer's identity, and silently changes the store's shape on restore.
- **Skip links and report them as omitted.** Rejected: the restored store would be a different store
  — a publication pointer is what makes the workbook observable, and a "complete" backup that cannot
  restore it is the defect being fixed.
- **Follow links anywhere for operator convenience.** Rejected: restore would then depend on,
  and absorb, paths the store does not own.
- **Replace the pointer with a regular file or directory.** Rejected: contradicts the single atomic
  pointer flip of ADR-026 and re-implements generation switching as data.
- **Bump the manifest version for the new field.** Rejected: the field is additive and optional;
  a bump would make every existing backup unreadable ("take the backup again with this build") for no
  safety gain, since an unknown link entry cannot be mistaken for a regular file after validation.

## Consequences

- A published store backs up and restores with its pointer intact; the restored store resolves
  `out/publication/current` beside itself, because the link is relative.
- A backup tree is now a tree of regular files, directories and store-relative links; tooling that
  walks a backup must decide explicitly what to do with links instead of assuming none exist.
- The refusal message for an escaping link differs from the old "all symlinks refused" message; the
  operator contract in [FJALL_BACKUP.md](../FJALL_BACKUP.md) is updated accordingly.

## Implementation status

Landed in `census-store`'s backup module: `tree.rs` (link copy and lexical containment),
`manifest.rs` (link entries), `restore.rs` (declared-target validation, link creation, `links`
accounting), `mod.rs`/`copy.rs` (report and manifest shape). Regression coverage in
`backup_tests.rs`: `backup_and_restore_carry_a_store_relative_publication_pointer`,
`backup_refuses_a_symlink_that_leaves_the_store` and `restore_refuses_a_tampered_link_entry`, plus
the retained absolute-link refusals. Dated command evidence is in
[VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md).
