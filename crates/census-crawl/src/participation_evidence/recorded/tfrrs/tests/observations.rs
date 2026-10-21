use super::*;
use crate::participation_evidence::hash;

pub(super) fn exact(store: &Store, key: &str, payload: &serde_json::Value) -> TestResult<Vec<u8>> {
    let before = fixture::state(store)?;
    let raw = fixture::raw(store, key)?;
    check!(eq; serde_json::from_str::<serde_json::Value>(&raw)?, *payload);
    let mut grant = AttachmentReplayGrantV1::new();
    let snapshot = store.snapshot();
    let held = read_recorded_tfrrs_list_capture(&snapshot, key, &fixture::locators(), &mut grant)?
        .ok_or("owned TFRRS capture")?;
    check!(eq; held.root().schema, fixture::SCHEMA);
    check!(eq; held.root().key, key);
    check!(eq; held.root().payload.digest.as_str(), hash(raw.as_bytes()));
    check!(eq; held.root().payload.bytes, u64::try_from(raw.len())?);
    check!(eq; serde_json::to_value(held.capture())?, *payload);
    let list = held.read(&fixture::locators())?;
    check!(eq; list.original_document().as_bytes(), fixture::BODY);
    check!(eq; list.context_reference(), key);
    check!(eq; list.list_id(), "5489");
    check!(eq; list.capture().body_sha256(), "053ff7832326a6d27e2456155db7a9530682393fae94a9fcb9ccc9e88bfc8407");
    let rows = exact_rows(&list)?;
    let usage = grant.usage();
    check!(eq; usage.encoded_bytes(), raw.len());
    check!(usage.traversal_steps() > 0);
    drop(list);
    drop(held);
    check!(eq; grant.usage(), usage);
    check!(eq; fixture::state(store)?, before);
    Ok(rows)
}

fn exact_rows(list: &CheckedRetainedTfrrsListV1<'_>) -> TestResult<Vec<u8>> {
    let mut retained = Vec::new();
    for (index, (athlete, meet, date, digest)) in [
        (
            "8650211",
            "95201",
            "2026-03-21",
            "c6809a7ceb8c2c8d70c5c8158b96eb13e3aaa881be19d866db75539cd2f5caf4",
        ),
        (
            "8792017",
            "93917",
            "2026-03-11",
            "595aa36734f340a2a4becf951002d8a5a2984adef7204d5f49abf52fd9e78ad4",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let row = list.row(index).ok_or("reopened source row")?;
        check!(eq; row.athlete_id(), athlete);
        check!(eq; row.meet_id(), meet);
        check!(eq; row.published_date(), date);
        check!(eq; row.original_row_digest(), digest);
        check!(eq; row.locator(), *fixture::locators().get(index).ok_or("exact locator")?);
        check!(eq; row.original_row_digest(), hash(row.original_row()?.as_bytes()));
        retained.push(serde_json::json!({"reference":row.reference(), "subject":row.subject()?.reference(),
            "context":row.context_reference(),"locator":row.locator(),"athlete":row.athlete_id(),
            "meet":row.meet_id(),"date":row.published_date(),"published":row.published_mark(),
            "original":row.original_mark(),"note":row.conversion_note(),"note_span":row.conversion_note_span()}));
    }
    Ok(serde_json::to_vec(&retained)?)
}
