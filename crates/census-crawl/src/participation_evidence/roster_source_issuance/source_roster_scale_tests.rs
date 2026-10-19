use super::*;
use census_domain::model::{SourceNamespace, SourceObservation, SourceSchoolObservation};

fn append_unrelated_observations(store: &census_store::Store) -> TestResult {
    let mut encoded = 0_u64;
    for index in 0..144 {
        let observation = SourceObservation::School(SourceSchoolObservation::new(
            SourceNamespace::Other("controlled-unrelated-school".to_owned()),
            format!("unrelated-scale-{index}"),
            format!("controlled-unrelated-row-{index}"),
            "x".repeat(60 * 1024),
            "2026-10-19",
        ));
        let length = u64::try_from(serde_json::to_vec(&observation)?.len())?;
        check!(length < 64 * 1024);
        encoded = encoded
            .checked_add(length)
            .ok_or("scale byte count overflow")?;
        store.append(Table::SourceObservations, &observation)?;
    }
    check!(encoded > 8 * 1024 * 1024);
    Ok(())
}

#[test]
fn reviewed_roster_retains_source_after_unrelated_census_growth() -> TestResult {
    let (directory, store) = issued()?;
    let mut before_grant = AttachmentReplayGrantV1::new();
    let before = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut before_grant)?;
    first(&before, &mut before_grant)?;
    let original = before
        .claims(&mut before_grant)?
        .first()
        .ok_or("reviewed claim")?
        .original_input_token(&mut before_grant)?
        .to_vec();
    let old_frozen = before.freeze_closure(&mut before_grant)?;
    append_unrelated_observations(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    first(&current, &mut grant)?;
    check!(eq; original.as_slice(), current.claims(&mut grant)?.first().ok_or("current claim")?
        .original_input_token(&mut grant)?);
    let frozen = current.freeze_closure(&mut grant)?;
    drop(store);
    directory.close()?;
    for retained in [&old_frozen, &frozen] {
        let mut replay_grant = AttachmentReplayGrantV1::new();
        let replay = replay_frozen_roster_reviewed_claims_v1(
            retained.bytes(),
            retained.reference(),
            &mut replay_grant,
        )?;
        let claim = replay
            .claims(&mut replay_grant)?
            .first()
            .ok_or("replayed claim")?;
        check!(eq; original.as_slice(), claim.original_input_token(&mut replay_grant)?);
        reassess(claim, &mut replay_grant)?;
    }
    Ok(())
}
