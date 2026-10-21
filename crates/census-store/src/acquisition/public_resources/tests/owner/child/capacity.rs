use super::*;
use std::time::Instant;

const CHILDREN: usize = 100;

#[test]
fn one_hundred_children_share_one_retained_parent_and_exhaust_exactly_across_cold_opens(
) -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let before = Witness::capture(&f.store)?;
    let started = Instant::now();
    let works = register_inventory(&f, &owner, &original)?;
    eprintln!(
        "registered {CHILDREN} controlled children in {:?}",
        started.elapsed()
    );
    check!(eq; works.len(), CHILDREN);
    let after = Witness::capture(&f.store)?;
    check!(eq; after.sequence, before.sequence.checked_add(u64::try_from(CHILDREN)?).ok_or("child inventory sequence")?);
    check!(eq; after.journal.len(), before.journal.len().checked_add(3 * CHILDREN).ok_or("child inventory journal rows")?);
    check!(eq; after.receipts.len(), before.receipts.len().checked_add(CHILDREN).ok_or("child inventory receipts")?);
    check!(eq; after.meta, before.meta);
    check!(eq; after.entities, before.entities);
    for work in &works {
        fixture::verify_registered(&f.store, work)?;
    }
    let last = CHILDREN.checked_sub(1).ok_or("last child ordinal")?;
    let assessed = inventory_assessment(&f.store, &owner, &original, last)?;
    let repeated = f
        .store
        .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
    check!(eq; repeated.work(), works.get(last).ok_or("hundredth registered child")?);
    check!(eq; Witness::capture(&f.store)?, after);
    overflow_refuses(&f, &owner, &original)?;
    check!(eq; Witness::capture(&f.store)?, after);
    drop((assessed, repeated, owner));
    cold_inventory(f, &original, &works, &after)
}

fn register_inventory(
    f: &Fixture,
    owner: &PublicResourceCaptureOwnerV1,
    original: &AcquisitionOccurrenceRefV1,
) -> TestResult<Vec<AcquisitionWorkV1>> {
    let mut works = Vec::new();
    works.try_reserve_exact(CHILDREN)?;
    for ordinal in 0..CHILDREN {
        let assessed = inventory_assessment(&f.store, owner, original, ordinal)?;
        let prepared = f.store.prepare_public_roster_child(&assessed)?;
        let expected = prepared.work().clone();
        let owned = f.store.register_public_roster_child(prepared)?;
        check!(eq; owned.work(), &expected);
        works.push(expected);
    }
    Ok(works)
}

fn inventory_assessment(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    original: &AcquisitionOccurrenceRefV1,
    ordinal: usize,
) -> TestResult<fixture::ControlledProtocolAssessment> {
    let mut assessed = assessment(store, owner, original)?;
    let native = 30_000_usize
        .checked_add(ordinal)
        .ok_or("controlled native team")?;
    assessed.team_id = native.to_string();
    assessed.team_url = format!("https://mi.milesplit.com/teams/{native}-controlled-protocol");
    assessed.roster_url = format!("{}/roster", assessed.team_url);
    assessed.parsed_teams = CHILDREN;
    Ok(assessed)
}

fn overflow_refuses(
    f: &Fixture,
    owner: &PublicResourceCaptureOwnerV1,
    original: &AcquisitionOccurrenceRefV1,
) -> TestResult {
    let assessed = inventory_assessment(&f.store, owner, original, CHILDREN)?;
    let prepared = f.store.prepare_public_roster_child(&assessed)?;
    let work = prepared.work().clone();
    let before = Witness::capture(&f.store)?;
    refused(
        f.store.register_public_roster_child(prepared),
        "public roster child directory inventory exhausted",
    )?;
    check!(eq; Witness::capture(&f.store)?, before);
    check!(!f.store.journal.contains_key(protocol::key(&work)?)?);
    Ok(())
}

fn cold_inventory(
    f: Fixture,
    original: &AcquisitionOccurrenceRefV1,
    works: &[AcquisitionWorkV1],
    expected: &Witness,
) -> TestResult {
    let parent = f
        .store
        .read_retained_acquisition_v1(original, &mut AcquisitionReadBudget::for_work())?;
    let metadata = parent.prepared().metadata_bytes().to_vec();
    let body = std::fs::read(parent.body_path())?;
    drop(parent);
    let root = f.directory.path().join("store");
    let Fixture {
        directory,
        store,
        writer,
        binding: _,
    } = f;
    store.flush()?;
    drop((store, writer));
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; &Witness::capture(&store)?, expected);
        for work in works {
            fixture::verify_registered(&store, work)?;
        }
        let parent =
            store.read_retained_acquisition_v1(original, &mut AcquisitionReadBudget::for_work())?;
        check!(eq; parent.reference(), original);
        check!(eq; parent.body_reference(), original);
        check!(eq; parent.prepared().metadata_bytes(), metadata.as_slice());
        check!(eq; std::fs::read(parent.body_path())?, body);
        check!(eq; &Witness::capture(&store)?, expected);
        drop((parent, store));
    }
    drop(directory);
    Ok(())
}
