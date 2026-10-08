use super::super::{files, proof, MARKER};
use super::{failure, fixture};
use anyhow::{Context, Result};
use census_service::restate_services::{
    StageOutcome, TeamsAttemptProgress, TeamsSourceInspection, TeamsStage,
};
use serde_json::{json, Value};

#[test]
fn missing_or_incompletely_published_marker_never_accepts_a_reset() -> Result<()> {
    let (original, observation) = fixture::active()?;
    let directory = fixture::directory()?;
    let config = fixture::configured(directory.path(), &original)?;
    assert!(fixture::runtime(&config, &original, &observation)?.is_none());
    let marker = config.with_file_name(MARKER);
    let pending = files::pending(&marker)?;
    fixture::write_private(&pending, &serde_json::to_vec(&fixture::marker(&original)?)?)?;
    assert!(fixture::runtime(&config, &original, &observation)?.is_none());
    std::fs::hard_link(&pending, &marker)?;
    assert!(fixture::runtime(&config, &original, &observation)?.is_none());
    std::fs::remove_file(&pending)?;
    assert_eq!(
        fixture::runtime(&config, &original, &observation)?
            .context("published witness absent")?
            .get("source_attempt"),
        Some(&json!(1))
    );
    Ok(())
}

#[test]
fn marker_refuses_each_mismatched_authority_field() -> Result<()> {
    [
        ("schema", json!(1), "schema/phase"),
        ("phase", json!("http_response"), "schema/phase"),
        (
            "operation",
            json!("another-parent/teams/milesplit"),
            "operation differs",
        ),
        ("attempt", json!(2), "attempt differs"),
        ("request_digest", json!("b".repeat(64)), "digest differs"),
        (
            "observed_on",
            json!("2026-10-03"),
            "observation date differs",
        ),
        ("extra", json!(true), "unknown field"),
    ]
    .into_iter()
    .try_for_each(|(field, value, diagnostic)| -> Result<()> {
        let (original, observation) = fixture::active()?;
        let directory = fixture::directory()?;
        let config = fixture::configured(directory.path(), &original)?;
        let mut marker = serde_json::to_value(fixture::marker(&original)?)?;
        marker
            .as_object_mut()
            .context("fixture marker malformed")?
            .insert(field.to_owned(), value);
        fixture::write_private(
            &config.with_file_name(MARKER),
            &serde_json::to_vec(&marker)?,
        )?;
        assert!(
            failure(fixture::runtime(&config, &original, &observation))?.contains(diagnostic),
            "{field}"
        );
        Ok(())
    })
}

#[test]
fn marker_cannot_override_completed_parent_or_completed_teams_stage() -> Result<()> {
    let (original, mut observation) = fixture::active()?;
    let directory = fixture::directory()?;
    let config = fixture::configured(directory.path(), &original)?;
    fixture::publish_marker(&config, &fixture::marker(&original)?)?;
    observation.parent_state.teams = TeamsStage::Completed(
        StageOutcome {
            records: 1,
            at: "2026-10-02".to_owned(),
            errors: Vec::new(),
            notes: Vec::new(),
        }
        .try_into()?,
    );
    assert_eq!(
        failure(fixture::runtime(&config, &original, &observation))?,
        "source boundary does not retain owed teams stage"
    );
    observation.parent_state.teams = TeamsStage::Owed;
    fixture::completed_parent(&mut observation)?;
    assert!(fixture::runtime(&config, &original, &observation)?.is_none());
    Ok(())
}

#[test]
fn marker_cannot_override_completed_child_or_consumed_attempt() -> Result<()> {
    let (original, mut observation) = fixture::active()?;
    let directory = fixture::directory()?;
    let config = fixture::configured(directory.path(), &original)?;
    fixture::publish_marker(&config, &fixture::marker(&original)?)?;
    let child = observation
        .children
        .first_mut()
        .context("fixture child absent")?;
    let advanced = TeamsSourceInspection::Unsettled {
        progress: vec![
            TeamsAttemptProgress::Unknown { attempt: 1 },
            TeamsAttemptProgress::Unknown { attempt: 2 },
        ],
    };
    child.inspection_before = advanced.clone();
    child.inspection_after = advanced;
    assert!(failure(fixture::runtime(&config, &original, &observation))?
        .contains("attempt differs from actual reservation"));
    ["before", "after"]
        .into_iter()
        .try_for_each(|bookend| -> Result<()> {
            let value = if bookend == "before" {
                &mut observation.before
            } else {
                &mut observation.after
            };
            let child = value
                .as_array_mut()
                .context("fixture statuses absent")?
                .iter_mut()
                .find(|row| row.get("id") == Some(&json!("inv_child")))
                .context("fixture child absent")?;
            child["status"] = json!("completed");
            Ok(())
        })?;
    assert!(fixture::runtime(&config, &original, &observation)?.is_none());
    Ok(())
}

#[test]
fn retained_marker_requires_byte_exact_unchanged_evidence_after_reboot() -> Result<()> {
    let (original, observation) = fixture::active()?;
    let directory = fixture::directory()?;
    let config = fixture::configured(directory.path(), &original)?;
    let marker = fixture::marker(&original)?;
    fixture::publish_marker(&config, &marker)?;
    let witness =
        fixture::runtime(&config, &original, &observation)?.context("fixture witness absent")?;
    let before = json!({"boundary":{"observation":observation,"witness":witness}});
    let retained = proof::retained_at(&config, &original, &before)?;
    assert_eq!(
        retained.get("original_invocation_id"),
        Some(&json!("inv_parent"))
    );
    assert_eq!(
        retained.get("retained"),
        before.pointer("/boundary/witness/native_source_boundary")
    );
    assert_eq!(retained.get("attempt_reset"), Some(&json!(false)));
    std::fs::write(
        config.with_file_name(MARKER),
        serde_json::to_vec_pretty(&marker)?,
    )?;
    assert!(failure(proof::retained_at(&config, &original, &before))?
        .contains("evidence changed across reboot"));
    Ok(())
}

#[test]
fn retained_marker_refuses_missing_authority_after_reboot() -> Result<()> {
    let (original, observation) = fixture::active()?;
    let directory = fixture::directory()?;
    let config = fixture::configured(directory.path(), &original)?;
    fixture::publish_marker(&config, &fixture::marker(&original)?)?;
    let witness =
        fixture::runtime(&config, &original, &observation)?.context("fixture witness absent")?;
    let before: Value = json!({"boundary":{"observation":observation,"witness":witness}});
    std::fs::remove_file(config.with_file_name(MARKER))?;
    assert!(failure(proof::retained_at(&config, &original, &before))?
        .contains("marker missing or incompletely published after reboot"));
    Ok(())
}
