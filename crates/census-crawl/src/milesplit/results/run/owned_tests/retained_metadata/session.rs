use super::*;
use crate::milesplit::retained_view::counting::CountingJournal;
use crate::milesplit::RetainedResultSession;

#[tokio::test]
async fn shared_capture_session_reads_and_parses_once_for_many_rows() -> TestResult {
    let body = many_rows()?;
    let fixture = Fixture::prepare(&body).await?;
    let snapshot = fixture.store.snapshot();
    let view = CountingJournal::new(&snapshot);
    let mut session = RetainedResultSession::new(&view, budget()?);
    let first = session.read(&fixture.reference, &fixture.raw, &fixture.owned, "data[0]")?;
    let capture_reads = view.reads_for(OWNED_CAPTURE_PHASE);
    let interpretation_reads = view.reads_for(crate::milesplit::OWNED_MEET_PHASE);
    (0..1000).try_for_each(|index| -> TestResult {
        let locator = format!("data[{index}]");
        let row = session.read(&fixture.reference, &fixture.raw, &fixture.owned, &locator)?;
        assert!(std::ptr::eq(first.capture(), row.capture()));
        assert!(std::ptr::eq(
            first.result().acquisition(),
            row.result().acquisition()
        ));
        assert!(std::ptr::eq(
            first.result().parsed_rows(),
            row.result().parsed_rows()
        ));
        assert_eq!(row.result().row().locator, locator);
        Ok(())
    })?;
    assert_eq!(view.reads_for(OWNED_CAPTURE_PHASE), capture_reads);
    assert_eq!(
        view.reads_for(crate::milesplit::OWNED_MEET_PHASE),
        interpretation_reads
            .checked_mul(1001)
            .ok_or("interpretation read count overflow")?
    );
    assert_eq!(session.resident_captures(), (1, 1));
    Ok(())
}

#[tokio::test]
async fn cached_capture_still_refuses_swapped_reference_and_locator() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let snapshot = fixture.store.snapshot();
    let mut session = RetainedResultSession::new(&snapshot, budget()?);
    session.read(&fixture.reference, &fixture.raw, &fixture.owned, "data[0]")?;
    for locator in ["data[00]", "data[19999]", "data[0]/other"] {
        assert!(session
            .read(&fixture.reference, &fixture.raw, &fixture.owned, locator)
            .is_err());
    }
    let mut foreign = fixture.reference.clone();
    foreign.rsid = "999999".into();
    assert!(session
        .read(&foreign, &fixture.raw, &fixture.owned, "data[0]")
        .is_err());
    let mut foreign = fixture.reference.clone();
    foreign.url = foreign.url.replace("milesplit.com", "example.com");
    assert!(session
        .read(&foreign, &fixture.raw, &fixture.owned, "data[0]")
        .is_err());
    assert_eq!(session.resident_captures(), (1, 1));
    Ok(())
}

#[tokio::test]
async fn interleaved_capture_groups_reuse_then_release_their_bodies() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let raw = std::str::from_utf8(FEMALE_RAW)?.replace("2026-03-27", "2026-03-28");
    let other = fixture.archive(raw.as_bytes(), None)?;
    let snapshot = fixture.store.snapshot();
    let view = CountingJournal::new(&snapshot);
    let mut session = RetainedResultSession::new(&view, budget()?);
    (0..100).try_for_each(|_| -> TestResult {
        session.read(&fixture.reference, &fixture.raw, &fixture.owned, "data[0]")?;
        session.read(&fixture.reference, &other, &fixture.owned, "data[0]")?;
        Ok(())
    })?;
    assert_eq!(session.resident_captures(), (1, 2));
    assert_eq!(
        view.reads_of(OWNED_CAPTURE_PHASE, fixture.owned.manifest_key()),
        1
    );
    assert_eq!(
        view.reads_of(OWNED_CAPTURE_PHASE, fixture.raw.manifest_key()),
        1
    );
    assert_eq!(view.reads_of(OWNED_CAPTURE_PHASE, other.manifest_key()), 1);
    session.clear();
    assert_eq!(session.resident_captures(), (0, 0));
    session.read(&fixture.reference, &fixture.raw, &fixture.owned, "data[0]")?;
    assert_eq!(session.resident_captures(), (1, 1));
    assert_eq!(
        view.reads_of(OWNED_CAPTURE_PHASE, fixture.owned.manifest_key()),
        2
    );
    Ok(())
}

fn many_rows() -> TestResult<Vec<u8>> {
    let mut document: Value = serde_json::from_slice(TROY)?;
    let original = document
        .get("data")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("source row")?
        .clone();
    let rows = (0_u64..1000)
        .map(|index| -> TestResult<Value> {
            let mut row = original.clone();
            let result_id = 300_000_000_u64
                .checked_add(index)
                .ok_or("result ID overflow")?;
            let athlete_id = 50_000_000_u64
                .checked_add(index)
                .ok_or("athlete ID overflow")?;
            let fields = row.as_object_mut().ok_or("row object")?;
            fields.insert("id".into(), json!(result_id));
            fields.insert("athleteId".into(), json!(athlete_id.to_string()));
            fields.insert(
                "profileUrl".into(),
                json!(format!("https://www.milesplit.com/athletes/{athlete_id}")),
            );
            Ok(row)
        })
        .collect::<TestResult<Vec<_>>>()?;
    document
        .as_object_mut()
        .ok_or("document object")?
        .insert("data".into(), Value::Array(rows));
    Ok(serde_json::to_vec(&document)?)
}

#[tokio::test]
async fn session_refuses_seventeenth_raw_capture_before_reading_it() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let raw = std::str::from_utf8(FEMALE_RAW)?;
    let archives = (0..17)
        .map(|index| {
            fixture.archive(
                format!("{raw}<!-- synthetic capture {index} -->").as_bytes(),
                None,
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    let snapshot = fixture.store.snapshot();
    let view = CountingJournal::new(&snapshot);
    let mut session = RetainedResultSession::new(&view, budget()?);
    for archive in archives.iter().take(16) {
        session.read(&fixture.reference, archive, &fixture.owned, "data[0]")?;
    }
    let refused = archives.last().ok_or("seventeenth capture")?;
    assert!(matches!(
        session.read(&fixture.reference, refused, &fixture.owned, "data[0]"),
        Err(RetainedMetadataError::SessionBudget)
    ));
    assert_eq!(session.resident_captures(), (1, 16));
    assert_eq!(
        view.reads_of(OWNED_CAPTURE_PHASE, refused.manifest_key()),
        0
    );
    session.clear();
    session.read(&fixture.reference, refused, &fixture.owned, "data[0]")?;
    assert_eq!(session.resident_captures(), (1, 1));
    Ok(())
}

#[tokio::test]
async fn cached_body_cannot_substitute_a_different_retained_interpretation() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let rows = crate::milesplit::parse_owned_meet(TROY, 725218)?;
    let second = rows.rows.get(1).ok_or("second row")?;
    let digest = census_domain::model::serialized_digest(&serde_json::to_value(second)?)?;
    let key = format!(
        "row/725218/{}/data[1]/{digest}",
        crate::net::cache::content_digest(TROY)
    );
    let mut batch = fixture.store.write_batch();
    batch.journal_done(
        crate::milesplit::OWNED_MEET_PHASE,
        &key,
        &json!({"foreign": "interpretation"}),
    )?;
    batch.commit()?;
    let snapshot = fixture.store.snapshot();
    let mut session = RetainedResultSession::new(&snapshot, budget()?);
    session.read(&fixture.reference, &fixture.raw, &fixture.owned, "data[0]")?;
    assert!(
        matches!(session.read(&fixture.reference, &fixture.raw, &fixture.owned, "data[1]"),
        Err(RetainedMetadataError::OwnedResult(crate::milesplit::RetainedResultError::ChangedInterpretation { key: actual })) if actual == key)
    );
    Ok(())
}
