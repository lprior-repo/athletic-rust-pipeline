use super::super::super::source_binding::SourceBindingObjectV1;
use super::*;
use fixture::{install, Capture, Fixture};
use std::collections::BTreeSet;

#[test]
fn missing_reviewed_rows_remain_unknown_with_exact_gaps_after_frozen_replay() -> TestResult {
    let fixture = Fixture::new()?;
    let captures = [
        install(&fixture, "12441", "42", Sport::IndoorTrack, true)?,
        install(&fixture, "12442", "43", Sport::OutdoorTrack, true)?,
    ];
    for capture in &captures {
        fixture.accept(&capture.athlete.id)?;
    }
    let dataset = fixture.dataset()?;
    let snapshot = fixture.store.snapshot();
    let objects = fixture.directory.path().join("missing-authority-objects");
    std::fs::create_dir(&objects)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let athletes = crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
        &dataset,
        &snapshot,
        Table::Athletes,
        &objects,
        fixture::budget(),
    )?;
    let bound = BoundRosterSourceViewV1::capture(
        &dataset,
        &fixture.profile,
        &objects,
        &claims,
        athletes.clone(),
        fixture::instant()?,
        &mut grant,
    )?;
    let (pinned, family) = super::super::assess(&dataset, &fixture.profile, &bound, &mut grant)?;
    assert_missing(
        &pinned.into_verified(),
        &serde_json::from_slice(family.bytes())?,
        &captures,
    )?;
    let family_ref = SourceBindingObjectV1 {
        digest: family.digest().into(),
        bytes: family.bytes().len().try_into()?,
    };
    let closure = bound.closure_reference().clone();
    drop(bound);
    drop(claims);
    drop(snapshot);
    let Fixture {
        directory: _directory,
        store,
        writer,
        profile,
    } = fixture;
    drop(writer);
    drop(store);
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let (replayed, replay_family) = BoundRosterSourceViewV1::with_replay(
        &dataset,
        &profile,
        &objects,
        &closure,
        &athletes,
        &family_ref,
        &mut replay_grant,
        |bound, grant| super::super::assess(&dataset, &profile, bound, grant),
    )?;
    assert_missing(
        &replayed.into_verified(),
        &serde_json::from_slice(replay_family.bytes())?,
        &captures,
    )?;
    let mut exhausted = AttachmentReplayGrantV1::new();
    let error = BoundRosterSourceViewV1::with_replay(
        &dataset,
        &profile,
        &objects,
        &closure,
        &athletes,
        &family_ref,
        &mut exhausted,
        |bound, grant| {
            let remaining = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
                .checked_sub(grant.usage().traversal_steps())
                .ok_or(StoreError::CounterOverflow)?;
            super::super::charge(grant, 0, remaining)?;
            super::super::assess(&dataset, &profile, bound, grant)
        },
    )
    .err()
    .ok_or("exhausted original grant returned an affiliation")?;
    check!(matches!(error, crate::report::ReportError::Store(_)));
    check!(matches!(
        exhausted.ensure_available(),
        Err(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

fn assert_missing(
    verified: &crate::export::VerifiedCurrentAffiliations,
    inventory: &Value,
    captures: &[Capture],
) -> TestResult {
    let mut expected = BTreeSet::new();
    for capture in captures {
        let reference = row_key(capture)?;
        check!(eq; verified.resolution(&capture.athlete.id), &Resolution::Unknown);
        let audit = verified
            .audit(&capture.athlete.id)
            .ok_or("missing authority audit")?;
        check!(audit.references().contains(&reference));
        check!(audit
            .unavailable()
            .iter()
            .any(|reason| reason == "archive_unavailable"));
        expected.insert((
            capture.athlete.id.as_str().to_string(),
            reference,
            "missing_reviewed_source_authority".to_string(),
        ));
    }
    let gaps = inventory
        .pointer("/2/gaps")
        .and_then(Value::as_array)
        .ok_or("physical gaps")?;
    let actual = gaps
        .iter()
        .map(|gap| -> TestResult<_> {
            Ok((
                gap.get("athlete")
                    .and_then(Value::as_str)
                    .ok_or("gap athlete")?
                    .to_string(),
                gap.get("reference")
                    .and_then(Value::as_str)
                    .ok_or("gap reference")?
                    .to_string(),
                gap.get("reason")
                    .and_then(Value::as_str)
                    .ok_or("gap reason")?
                    .to_string(),
            ))
        })
        .collect::<TestResult<BTreeSet<_>>>()?;
    check!(eq; gaps.len(), expected.len());
    check!(eq; actual, expected);
    check!(eq; inventory.pointer("/2/supported"), Some(&json!([])));
    check!(eq; inventory.pointer("/2/witnesses"), Some(&json!([])));
    Ok(())
}

fn row_key(capture: &Capture) -> TestResult<String> {
    for evidence in &capture.athlete.evidence {
        if let Some(note) = super::super::super::note::read(evidence)? {
            return Ok(note.source_row_key().to_string());
        }
    }
    Err("controlled source has no original row reference".into())
}
