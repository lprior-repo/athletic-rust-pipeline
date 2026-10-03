use super::super::{configuration, files, operation, MARKER};
use super::{failure, fixture};
use anyhow::Result;
use census_domain::UsJurisdiction;
use serde_json::json;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

#[test]
fn config_uses_original_request_revision_without_identity_exceptions() -> Result<()> {
    [17, 43].into_iter().try_for_each(|revision| -> Result<()> {
        let original = fixture::original(UsJurisdiction::RhodeIsland, revision)?;
        let directory = fixture::directory()?;
        let config = fixture::configured(directory.path(), &original)?;
        assert_eq!(fixture::read_config(&config)?, json!({"schema":1,"operation":format!("jurisdiction:RI:2026-27:{revision}/teams/milesplit"),"attempt":1,"timeout_seconds":60}));
        assert_eq!(std::fs::symlink_metadata(&config)?.mode() & 0o777, 0o600);
        assert_eq!(files::read_required(&config)?, serde_json::to_vec(&configuration(&original.request, &original.source_plan)?)?);
        assert!(!files::pending(&config)?.exists());
        Ok(())
    })
}

#[test]
fn config_refuses_reuse_without_replacing_the_original_bytes() -> Result<()> {
    let (original, _) = fixture::active()?;
    let directory = fixture::directory()?;
    let path = fixture::configured(directory.path(), &original)?;
    let original_bytes = files::read_required(&path)?;
    let error = failure(super::super::prepare_at(
        &path,
        &configuration(&original.request, &original.source_plan)?,
    ))?;
    assert!(error.contains("refuses reused or conflicting artifact"));
    assert_eq!(files::read_required(&path)?, original_bytes);
    Ok(())
}

#[test]
fn config_refuses_pending_or_marker_artifacts_before_publishing_authority() -> Result<()> {
    [
        "native-source-boundary-config.json.pending",
        MARKER,
        "teams-source-reservation-reached.json.pending",
    ]
    .into_iter()
    .try_for_each(|name| -> Result<()> {
        let (original, _) = fixture::active()?;
        let directory = fixture::directory()?;
        let conflicting = directory.path().join(name);
        fixture::write_private(&conflicting, b"preserved-artifact")?;
        let error = failure(fixture::configured(directory.path(), &original))?;
        assert!(error.contains("refuses reused or conflicting artifact"));
        assert_eq!(std::fs::read(&conflicting)?, b"preserved-artifact".to_vec());
        assert!(!directory
            .path()
            .join("native-source-boundary-config.json")
            .exists());
        Ok(())
    })
}

#[test]
fn config_refuses_a_public_directory_instead_of_weakening_privacy() -> Result<()> {
    let (original, _) = fixture::active()?;
    let directory = fixture::directory()?;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755))?;
    let error = failure(fixture::configured(directory.path(), &original))?;
    assert!(error.contains("owned private nonsymlink directory"));
    assert_eq!(std::fs::metadata(directory.path())?.mode() & 0o777, 0o755);
    Ok(())
}

#[test]
fn config_refuses_an_unplanned_source_and_mismatched_original_identity() -> Result<()> {
    let (mut original, _) = fixture::active()?;
    original
        .source_plan
        .sweepable
        .retain(|source| source != "milesplit");
    assert_eq!(
        failure(configuration(&original.request, &original.source_plan))?,
        "native boundary source is absent from the original plan"
    );
    original.key = "another-parent".to_owned();
    assert_eq!(
        failure(operation(&original))?,
        "native boundary original request identity differs"
    );
    Ok(())
}

#[test]
fn runtime_refuses_changed_configuration_instead_of_selecting_another_attempt() -> Result<()> {
    [
        ("schema", json!(2)),
        ("operation", json!("another-original/teams/milesplit")),
        ("attempt", json!(0)),
        ("attempt", json!(2)),
        ("timeout_seconds", json!(0)),
        ("timeout_seconds", json!(61)),
        ("extra", json!(true)),
    ]
    .into_iter()
    .try_for_each(|(field, replacement)| -> Result<()> {
        let (original, observation) = fixture::active()?;
        let directory = fixture::directory()?;
        let path = fixture::configured(directory.path(), &original)?;
        let mut changed = fixture::read_config(&path)?;
        changed
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("fixture config malformed"))?
            .insert(field.to_owned(), replacement);
        std::fs::write(&path, serde_json::to_vec(&changed)?)?;
        let error = failure(fixture::runtime(&path, &original, &observation))?;
        assert!(
            error.contains("config differs from original request")
                || error.contains("unknown field"),
            "{field}"
        );
        Ok(())
    })
}
