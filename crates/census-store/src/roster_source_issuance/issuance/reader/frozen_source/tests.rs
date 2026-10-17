use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn prefix(key: char) -> TestResult<CapturedExternalPrefix> {
    Ok(CapturedExternalPrefix {
        root: serde_json::from_value(serde_json::json!({
            "schema":"provider_school_packet_v1", "key":key.to_string().repeat(64),
            "payload":{"digest":"a".repeat(64), "bytes":1}
        }))?,
        rows: Vec::new(),
    })
}

fn row(group: &CapturedExternalPrefix, suffix: &[u8]) -> CapturedJournalRow {
    let mut key = crate::Store::journal_key(group.root.schema.phase(), &group.root.key);
    key.extend_from_slice(suffix);
    CapturedJournalRow {
        key,
        value: b"null".to_vec(),
    }
}

#[test]
fn complete_empty_prefix_is_missing_but_unknown_prefix_is_not() -> TestResult {
    let groups = vec![prefix('a')?];
    let mut grant = AttachmentReplayGrantV1::new();
    let source = FrozenRosterJournalSource::new(&groups, &mut grant)?;
    check!(source
        .read_root(groups[0].root.schema, &groups[0].root.key, &mut grant)?
        .is_none());
    check!(source
        .journal_get(&crate::Store::journal_key(
            groups[0].root.schema.phase(),
            &"b".repeat(64)
        ))
        .is_err());
    check!(source
        .visit_receipt_prefix(
            b"roster_source_issuance/",
            ReadBudget {
                rows: 1,
                bytes: 1024,
                record_bytes: 1024,
            },
            &mut (),
            |_| Ok(()),
            |_, _, _| Ok(())
        )
        .is_err());
    Ok(())
}

#[test]
fn duplicate_unordered_and_foreign_rows_refuse() -> TestResult {
    for mode in 0..3 {
        let mut group = prefix('a')?;
        let first = row(&group, &[]);
        let second = row(&group, if mode == 0 { &[] } else { &[1, 0, 0, 0, 0] });
        group.rows = if mode == 1 {
            vec![second, first]
        } else {
            vec![first, second]
        };
        if mode == 2 {
            group.rows[1].key = b"foreign\0row".to_vec();
        }
        let mut grant = AttachmentReplayGrantV1::new();
        check!(FrozenRosterJournalSource::new(&[group], &mut grant).is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn duplicate_prefixes_and_exhausted_or_foreign_grants_refuse() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    check!(FrozenRosterJournalSource::new(&[prefix('a')?, prefix('a')?], &mut grant).is_err());
    let groups = vec![prefix('a')?];
    let mut original = AttachmentReplayGrantV1::new();
    let source = FrozenRosterJournalSource::new(&groups, &mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(source
        .read_root(groups[0].root.schema, &groups[0].root.key, &mut foreign)
        .is_err());
    check!(foreign.ensure_available().is_err());
    let mut exhausted = AttachmentReplayGrantV1::new();
    exhausted.charge_bytes(
        census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes,
    )?;
    check!(FrozenRosterJournalSource::new(&groups, &mut exhausted).is_err());
    Ok(())
}

#[test]
fn covered_malformed_envelope_is_not_silently_missing() -> TestResult {
    let mut group = prefix('a')?;
    group.rows.push(row(&group, &[]));
    let groups = vec![group];
    let mut grant = AttachmentReplayGrantV1::new();
    let source = FrozenRosterJournalSource::new(&groups, &mut grant)?;
    check!(source
        .read_root(groups[0].root.schema, &groups[0].root.key, &mut grant)
        .is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}
