use super::{ArtifactStore, StoreError, MAX_BATCH_RECORDS, MAX_DOCUMENT_BYTES};
use crate::{
    domain::identity::{EvidenceDigest, SourceRowKey, WorkbookDigest},
    model::SourceRecord,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

fn workbook(digit: char) -> WorkbookDigest {
    WorkbookDigest::parse(&digit.to_string().repeat(64)).expect("workbook digest")
}

fn record(sheet: &str, row: u32, marker: &str) -> SourceRecord {
    let mut fields = BTreeMap::new();
    fields.insert("Person First".to_owned(), marker.to_owned());
    SourceRecord {
        source_key: format!("{sheet}:{row}"),
        sheet: sheet.to_owned(),
        excel_row: row,
        fields,
    }
}

fn open_store() -> (tempfile::TempDir, ArtifactStore) {
    let dir = tempdir().expect("temporary store path");
    let store = ArtifactStore::open(&dir.path().join("artifacts")).expect("open store");
    (dir, store)
}

#[test]
fn documents_survive_reopen_and_verify_digest() {
    let (dir, store) = open_store();
    let digest = store.put_bytes(b"fictional source").expect("put document");
    assert_eq!(
        store.get_bytes(&digest).expect("read document"),
        b"fictional source"
    );
    drop(store);
    let reopened = ArtifactStore::open(&dir.path().join("artifacts")).expect("reopen store");
    assert_eq!(
        reopened.get_bytes(&digest).expect("read reopened"),
        b"fictional source"
    );
}

#[test]
fn tampered_document_is_rejected() {
    let (_dir, store) = open_store();
    let digest = store.put_bytes(b"original").expect("put document");
    let key = super::keys::document_key(&digest);
    store
        .inner
        .documents
        .insert(key, b"tampered".to_vec())
        .expect("tamper fixture");
    assert!(matches!(
        store.get_bytes(&digest),
        Err(StoreError::DigestMismatch)
    ));
}

#[test]
fn duplicate_publication_is_idempotent_but_changed_row_conflicts() {
    let (_dir, store) = open_store();
    let wb = workbook('a');
    let first = record("Sheet", 2, "A");
    store
        .put_source_batch(&wb, std::slice::from_ref(&first))
        .expect("first import");
    store
        .put_source_batch(&wb, std::slice::from_ref(&first))
        .expect("duplicate import");
    let changed = record("Sheet", 2, "B");
    assert!(matches!(
        store.put_source_batch(&wb, std::slice::from_ref(&changed)),
        Err(StoreError::SourceConflict)
    ));
}

#[test]
fn source_rows_are_isolated_and_ordered_by_sheet_then_numeric_row() {
    let (_dir, store) = open_store();
    let first = record("Sheet", 10, "ten");
    let second = record("Sheet", 2, "two");
    let third = record("Other", 2, "other");
    let wb_a = workbook('a');
    let wb_b = workbook('b');
    store
        .put_source_batch(&wb_a, &[first, second, third])
        .expect("import rows");
    store
        .put_source_batch(&wb_b, &[record("Sheet", 2, "isolated")])
        .expect("import other workbook");
    let mut seen = Vec::new();
    store
        .visit_source(&wb_a, |row| {
            seen.push(row.source_key);
            Ok(())
        })
        .expect("visit rows");
    assert_eq!(seen, ["Other:2", "Sheet:2", "Sheet:10"]);
    let key = SourceRowKey::parse("Sheet:2").expect("row key");
    assert_eq!(
        store
            .source_record(&wb_b, &key)
            .expect("read isolated row")
            .map(|row| row.fields["Person First"].clone()),
        Some("isolated".to_owned())
    );
}

#[test]
fn source_pages_resume_after_gaps_without_crossing_sheet_boundaries() {
    let (_dir, store) = open_store();
    let wb = workbook('a');
    let rows = [
        record("Sheet", 2, "first"),
        record("Sheet", 10, "gap"),
        record("Sheet1", 3, "other"),
    ];
    store
        .put_source_batch(&wb, &rows)
        .expect("import sparse rows");
    let first = store
        .source_key_page(&wb, "Sheet", 0, 1)
        .expect("first page");
    assert_eq!(
        first.iter().map(SourceRowKey::as_str).collect::<Vec<_>>(),
        ["Sheet:2"]
    );
    let second = store
        .source_key_page(&wb, "Sheet", 2, 1)
        .expect("next page");
    assert_eq!(
        second.iter().map(SourceRowKey::as_str).collect::<Vec<_>>(),
        ["Sheet:10"]
    );
    assert!(store
        .source_key_page(&wb, "Sheet", 10, 1)
        .expect("end")
        .is_empty());
    assert!(store
        .source_key_page(&workbook('b'), "Sheet", 0, 1)
        .expect("other workbook")
        .is_empty());
    assert!(matches!(
        store.source_key_page(&wb, "Sheet", 0, 0),
        Err(StoreError::InvalidPageLimit)
    ));
}

#[test]
fn bounds_and_visitor_failures_are_returned() {
    let (_dir, store) = open_store();
    let too_large = vec![b'x'; MAX_DOCUMENT_BYTES + 1];
    assert!(matches!(
        store.put_bytes(&too_large),
        Err(StoreError::DocumentTooLarge)
    ));
    let rows = (2..=(MAX_BATCH_RECORDS as u32 + 2))
        .map(|row| record("Sheet", row, "x"))
        .collect::<Vec<_>>();
    let wb = workbook('a');
    assert!(matches!(
        store.put_source_batch(&wb, &rows),
        Err(StoreError::BatchTooLarge)
    ));
    let row = record("Sheet", 2, "x");
    store
        .put_source_batch(&wb, &[row])
        .expect("import callback row");
    let failure = store.visit_source(&wb, |_| Err(StoreError::Visitor));
    assert!(matches!(failure, Err(StoreError::Visitor)));
}

#[test]
fn raw_fields_are_retained_without_normalization() {
    let (_dir, store) = open_store();
    let wb = workbook('a');
    let mut row = record("Sheet", 2, "  exact  ");
    row.fields
        .insert("Unrecognized Header".to_owned(), "  keep me  ".to_owned());
    store
        .put_source_batch(&wb, &[row])
        .expect("import raw fields");
    let key = SourceRowKey::parse("Sheet:2").expect("row key");
    let stored = store
        .source_record(&wb, &key)
        .expect("read raw fields")
        .expect("present");
    assert_eq!(stored.fields["Person First"], "  exact  ");
    assert_eq!(stored.fields["Unrecognized Header"], "  keep me  ");
}

#[test]
fn absent_artifacts_are_distinct_from_corruption() {
    let (_dir, store) = open_store();
    let digest = EvidenceDigest::parse(&"b".repeat(64)).expect("digest");
    assert!(matches!(
        store.get_bytes(&digest),
        Err(StoreError::MissingArtifact)
    ));
}

#[cfg(unix)]
#[test]
fn rejects_existing_root_with_group_or_other_access() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().expect("temporary parent");
    let root = dir.path().join("store");
    std::fs::create_dir(&root).expect("create root");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
        .expect("set fixture permissions");
    assert!(matches!(
        ArtifactStore::open(&root),
        Err(StoreError::InsecureRoot)
    ));
}
