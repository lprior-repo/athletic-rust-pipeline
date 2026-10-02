use super::*;
use census_domain::model::{CanonicalMeet, Evidence, Gender, SourceEventLabel, SourceRef};
use census_domain::UsJurisdiction;

fn retained(label: &str, meet_name: &str) -> CanonicalEvent {
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-04-21",
        meet_name,
        None,
    );
    let source = SourceRef::new(
        "captured_results",
        Some("https://wi.milesplit.com/meets/739060/results/1285723/raw".into()),
    );
    let mut event = CanonicalEvent::new(
        &meet,
        EventKind::Unmapped {
            label: label.into(),
        },
        Gender::Boys,
        Some("2A"),
        Some("Preliminaries"),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    event.evidence.push(Evidence::parsed(source, "2026-09-28"));
    event
}

fn observations(store: &Store) -> StoreResult<Vec<CanonicalEvent>> {
    let mut rows = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Events, |row| {
            rows.push(row);
            Ok(())
        })?;
    Ok(rows)
}

fn report_counts(report: RepairReport) -> (u64, u64, u64, u64, u64) {
    (
        report.scanned,
        report.unmapped,
        report.eligible,
        report.corrected,
        report.unsupported_or_unbound,
    )
}

fn applied_counts(store: &Store) -> StoreResult<(u64, u64, u64, u64, u64)> {
    process_retained_events(store, RepairMode::Apply).map(report_counts)
}

#[test]
fn reported_hurdles_refine_without_changing_identity_context_or_source_labels() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = retained("Boys 2A 110m Hurdles Preliminaries", "Captured Meet");
    store.append_many(Table::Events, &[original.clone()])?;
    let report = process_retained_events(&store, RepairMode::Apply)?;
    assert_eq!(report_counts(report), (1, 1, 1, 1, 0));
    let rows = store.scan::<CanonicalEvent>(Table::Events)?;
    let corrected = rows.first().context("corrected event")?;
    assert_eq!(corrected.kind, EventKind::Track110mHurdles);
    let derived = corrected.evidence.last().context("derived evidence")?;
    let parsed = original.evidence.first().context("parsed evidence")?;
    assert_eq!(derived.method, EvidenceMethod::Derived);
    assert_eq!(derived.source, parsed.source);
    assert_eq!(derived.observed_on, parsed.observed_on);
    let mut preserved = corrected.clone();
    preserved.kind = original.kind.clone();
    preserved.evidence = original.evidence.clone();
    assert_eq!(preserved, original);
    assert_eq!(corrected.evidence.len(), original.evidence.len() + 1);
    assert_eq!(corrected.evidence.first(), original.evidence.first());
    let history = observations(&store)?;
    assert_eq!(history.len(), 2);
    assert!(history.contains(&original));
    assert!(history.contains(corrected));
    Ok(())
}

#[test]
fn dry_run_preserves_every_observation_and_table_digest() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = retained("Boys Varsity Shot Put Finals", "Captured Meet");
    store.append_many(Table::Events, &[original.clone()])?;
    let before = store.snapshot().tables_digest(&[Table::Events])?;
    let dry = process_retained_events(&store, RepairMode::DryRun)?;
    assert_eq!(report_counts(dry), (1, 1, 1, 0, 0));
    assert_eq!(store.snapshot().tables_digest(&[Table::Events])?, before);
    assert_eq!(
        store.scan::<CanonicalEvent>(Table::Events)?,
        [original.clone()]
    );
    assert_eq!(observations(&store)?, [original]);
    Ok(())
}

#[test]
fn field_and_relay_labels_with_multiple_agreeing_sources_are_refined() -> Result<()> {
    for (label, alias, kind) in [
        (
            "Boys Varsity Shot Put Finals",
            "shotput",
            EventKind::ShotPut,
        ),
        (
            "Girls Results Triple Jump Finals",
            "triplejump",
            EventKind::TripleJump,
        ),
        (
            "Boys Varsity 4x400 Meter Relay Finals",
            "4x400m",
            EventKind::Relay4x400,
        ),
    ] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let mut original = retained(label, "Captured Meet");
        let alternate = SourceRef::new(
            "other_results",
            Some("https://results.example.org/meet/1".into()),
        );
        original.source_labels.push(SourceEventLabel {
            source: alternate.clone(),
            label: alias.into(),
        });
        original
            .evidence
            .insert(0, Evidence::parsed(alternate, "2026-09-27"));
        store.append_many(Table::Events, &[original.clone()])?;
        let report = process_retained_events(&store, RepairMode::Apply)?;
        assert_eq!(report_counts(report), (1, 1, 1, 1, 0), "{label}");
        let rows = store.scan::<CanonicalEvent>(Table::Events)?;
        let corrected = rows.first().context("corrected event")?;
        assert_eq!(corrected.kind, kind);
        assert_eq!(
            corrected.evidence.last().map(|row| &row.source),
            original.evidence.get(1).map(|row| &row.source)
        );
        assert_eq!(
            corrected
                .evidence
                .last()
                .map(|row| row.observed_on.as_str()),
            Some("2026-09-28")
        );
        let mut preserved = corrected.clone();
        preserved.kind = original.kind.clone();
        preserved.evidence = original.evidence.clone();
        assert_eq!(preserved, original);
    }
    Ok(())
}

#[test]
fn unsupported_contradictory_and_unbound_events_remain_retained() -> Result<()> {
    for scenario in 0..9 {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let mut original = retained("110m Hurdles", "Captured Meet");
        match scenario {
            0 => original = retained("Unsupported Obstacle Race", "Captured Meet"),
            1 => original.source_labels.clear(),
            2 => original.evidence.clear(),
            3 => original
                .evidence
                .iter_mut()
                .for_each(|row| row.method = EvidenceMethod::Fetched),
            4 => original
                .source_labels
                .iter_mut()
                .for_each(|row| row.label = "110mh".into()),
            5 => original.source_labels.push(SourceEventLabel {
                source: original
                    .source_labels
                    .first()
                    .context("source label")?
                    .source
                    .clone(),
                label: "100m".into(),
            }),
            6 => original.source_labels.push(SourceEventLabel {
                source: SourceRef::new(
                    "unbound",
                    Some("https://results.example.org/meet/2".into()),
                ),
                label: "110mh".into(),
            }),
            7 => {
                original
                    .source_labels
                    .iter_mut()
                    .for_each(|row| row.source.url = None);
                original
                    .evidence
                    .iter_mut()
                    .for_each(|row| row.source.url = None);
            }
            _ => original
                .evidence
                .iter_mut()
                .for_each(|row| row.source.id = "different_capture".into()),
        }
        store.append_many(Table::Events, &[original.clone()])?;
        let report = process_retained_events(&store, RepairMode::Apply)?;
        assert_eq!(
            report_counts(report),
            (1, 1, 0, 0, 1),
            "scenario {scenario}"
        );
        assert_eq!(
            store.scan::<CanonicalEvent>(Table::Events)?,
            [original.clone()]
        );
        assert_eq!(observations(&store)?, [original]);
    }
    Ok(())
}

#[test]
fn applied_corrections_remain_idempotent_after_store_reopen() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let original = retained("110m Hurdles", "Captured Meet");
    store.append_many(Table::Events, &[original.clone()])?;
    assert_eq!(applied_counts(&store)?, (1, 1, 1, 1, 0));
    let before = observations(&store)?;
    drop(store);
    let reopened = Store::open(dir.path())?;
    assert_eq!(applied_counts(&reopened)?, (1, 0, 0, 0, 0));
    assert_eq!(observations(&reopened)?, before);
    let rows = reopened.scan::<CanonicalEvent>(Table::Events)?;
    assert_eq!(
        rows.first().map(|row| &row.kind),
        Some(&EventKind::Track110mHurdles)
    );
    assert!(observations(&reopened)?.contains(&original));
    Ok(())
}

#[test]
fn full_and_partial_batches_preserve_every_original_and_skip_known_events() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let originals: Vec<_> = (0..101)
        .map(|index| retained("110m Hurdles", &format!("Captured Meet {index}")))
        .collect();
    store.append_many(Table::Events, &originals)?;
    let mut known = retained("shotput", "Already Mapped Meet");
    known.kind = EventKind::ShotPut;
    store.append_many(Table::Events, &[known.clone()])?;
    assert_eq!(applied_counts(&store)?, (102, 101, 101, 101, 0));
    let rows = store.scan::<CanonicalEvent>(Table::Events)?;
    assert_eq!(rows.len(), 102);
    for original in &originals {
        let corrected = rows
            .iter()
            .find(|row| row.id == original.id)
            .context("corrected identity")?;
        let mut preserved = corrected.clone();
        preserved.kind = original.kind.clone();
        preserved.evidence = original.evidence.clone();
        assert_eq!(preserved, *original);
        assert_eq!(corrected.kind, EventKind::Track110mHurdles);
    }
    assert!(rows.contains(&known));
    let history = observations(&store)?;
    assert_eq!(history.len(), 203);
    assert!(originals.iter().all(|row| history.contains(row)));
    assert_eq!(applied_counts(&store)?, (102, 0, 0, 0, 0));
    assert_eq!(observations(&store)?, history);
    Ok(())
}

#[test]
fn checked_counters_fail_without_wrapping() -> Result<()> {
    let mut count = u64::MAX;
    let error = bump(&mut count).err().context("overflow must fail")?;
    assert!(matches!(error, StoreError::CounterOverflow));
    assert_eq!(count, u64::MAX);
    Ok(())
}
