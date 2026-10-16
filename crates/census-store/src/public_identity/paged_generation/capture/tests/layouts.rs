use super::*;
use census_domain::model::{
    CanonicalPerformance, EventId, Mark, MeetId, PerformanceIdentity, PerformanceResult,
    RelayResult, ReviewCase, SourceAthleteObservation, SourceNamespace, SourceObservation, TeamId,
};

type Physical = (AuthorityTableV2, Vec<u8>, Vec<u8>);

fn physical<T: Entity>(table: AuthorityTableV2, row: &T, generation: u64) -> TestResult<Physical> {
    let key = match table {
        AuthorityTableV2::ReviewCases => {
            crate::keys::derived_key(table.table(), generation, row.entity_id().as_bytes())
        }
        _ => crate::keys::observation_key(table.table(), row.entity_id(), 0),
    };
    let value =
        format!(" \n{}\t", serde_json::to_string_pretty(row)?).replace("4711", "\\u0034711");
    Ok((table, key, value.into_bytes()))
}

fn six_rows(generation: u64) -> TestResult<[Physical; 6]> {
    let athlete = subject("six layouts é", "4711")?;
    let observation = SourceObservation::Athlete(SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "4711",
        "capture-row",
        "Runner é",
        "2026-10-15",
    ));
    let case = ReviewCase::pending(
        "identity",
        athlete.id.as_str(),
        "Runner é",
        "Retain raw review",
    );
    let verdict = super::super::super::tests::verdict(&case.id);
    let meet = MeetId::mint("meet", &["capture-six"]);
    let event = EventId::mint("event", &["capture-six"]);
    let team = TeamId::mint("team", &["capture-six"]);
    let performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event,
            meet: &meet,
            date: "2026-10-15",
            source_key: "capture-six",
        },
        PerformanceResult {
            team: &team,
            mark: Mark::Raw("12.34".into()),
            wind_mps: None,
            place: Some(1),
            adjustment: None,
        },
    )?;
    let relay = RelayResult::new(
        &team,
        &event,
        &meet,
        Mark::Raw("49.19".into()),
        "capture-six",
    );
    Ok([
        physical(AuthorityTableV2::Athletes, &athlete, generation)?,
        physical(
            AuthorityTableV2::SourceObservations,
            &observation,
            generation,
        )?,
        physical(AuthorityTableV2::ReviewCases, &case, generation)?,
        physical(AuthorityTableV2::IdentityVerdicts, &verdict, generation)?,
        physical(AuthorityTableV2::Performances, &performance, generation)?,
        physical(AuthorityTableV2::RelayResults, &relay, generation)?,
    ])
}

fn set_rows(store: &Store, rows: &[Physical; 6]) -> TestResult {
    let mut batch = store.db.batch();
    for (table, key, value) in rows {
        batch.insert(&store.entities, key.as_slice(), value.as_slice());
        for prefix in ["sequence", "rows"] {
            crate::meta::put_text(
                &mut batch,
                &store.meta,
                &format!("{prefix}:{}", table.table().file()),
                "1",
            );
        }
    }
    let generation = store
        .snapshot()
        .derived_generation()
        .checked_add(1)
        .ok_or("generation")?;
    let case = ReviewCase::pending("identity", "future", "Future excluded", "Not published");
    batch.insert(
        &store.entities,
        crate::keys::derived_key(Table::ReviewCases, generation, case.id.as_bytes()),
        serde_json::to_vec(&case)?,
    );
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn check_six(
    store: &Store,
    retained: &RetainedRawGenerationV2,
    rows: &[Physical; 6],
) -> TestResult {
    let snapshot = store.snapshot();
    let seal = retained.verify(&snapshot)?;
    assert_eq!(seal.declared_inventory().rows(), 6);
    for ((kind, key, value), table) in rows.iter().zip(seal.tables()) {
        assert_eq!(table.table(), *kind);
        assert_eq!(table.inventory().rows(), 1);
        let binding = retained.binding().table_binding(*kind)?;
        let table = snapshot.raw_generation_table(&table.object(), &binding)?;
        let block = table.blocks().first().ok_or("block")?;
        let block = snapshot.raw_generation_block(&block.object(), &binding)?;
        let page = block.pages().first().ok_or("page")?;
        let page = snapshot.raw_generation_page(&page.object(), &binding)?;
        let row = page.rows().first().ok_or("row")?;
        assert_eq!(row.physical_key(), key);
        assert_eq!(row.raw_value(), value);
    }
    Ok(())
}

#[test]
fn six_layouts_capture_exact_bytes_and_replay_after_source_absent_reopen() -> TestResult {
    let (directory, store, writer) = enrolled()?;
    let rows = six_rows(store.snapshot().derived_generation())?;
    set_rows(&store, &rows)?;
    let id = operation("six-layouts")?;
    let captured = writer.capture_raw_generation_v2(&store, &id)?;
    assert!(matches!(
        &captured,
        RawGenerationCaptureOutcomeV2::Captured(_)
    ));
    check_six(&store, captured.retained(), &rows)?;
    let mut batch = store.db.batch();
    for (table, key, _) in &rows {
        batch.remove(&store.entities, key.as_slice());
        for prefix in ["sequence", "rows"] {
            crate::meta::put_text(
                &mut batch,
                &store.meta,
                &format!("{prefix}:{}", table.table().file()),
                "0",
            );
        }
    }
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let replay = writer.capture_raw_generation_v2(&store, &id)?;
    assert!(matches!(
        &replay,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    assert_eq!(replay.retained(), captured.retained());
    check_six(&store, replay.retained(), &rows)?;
    Ok(())
}
