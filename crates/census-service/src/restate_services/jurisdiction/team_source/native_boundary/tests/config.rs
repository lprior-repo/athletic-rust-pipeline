use std::fs::Permissions;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::time::Duration;

use super::super::config::Selection;
use super::{configuration, fixture, BoundaryError, Fixture, TestResult, OPERATION};

#[test]
fn valid_minimum_and_maximum_bounds_select_only_the_exact_reservation() -> TestResult {
    for attempt in [1, 3] {
        for seconds in [1, 60] {
            let fixture = fixture(attempt, seconds)?;
            let armed = fixture.armed()?;
            check!(eq; armed.config.select(OPERATION, attempt),
            Selection::Hold(Duration::from_secs(seconds)));
            check!(eq; armed
                .config
                .select("jurisdiction:RI:2026-27:1/teams/riil", attempt),
            Selection::Continue);
            check!(eq; armed.config.select(OPERATION, 2), Selection::Continue);
        }
    }
    Ok(())
}

#[test]
fn attempt_and_timeout_values_outside_the_frozen_budget_fail_closed() -> TestResult {
    for attempt in [0, 4, u8::MAX] {
        let fixture = fixture(attempt, 1)?;
        check!(matches!(fixture.armed(), Err(BoundaryError::Attempt(value)) if value == attempt));
    }
    for seconds in [0, 61, u64::MAX] {
        let fixture = fixture(1, seconds)?;
        check!(
            matches!(fixture.armed(), Err(BoundaryError::TimeoutBounds(value)) if value == seconds)
        );
    }
    Ok(())
}

#[test]
fn unsupported_schemas_and_unknown_or_duplicate_fields_fail_closed() -> TestResult {
    for schema in [0, 2, u8::MAX] {
        let mut config = configuration(1, 1);
        config["schema"] = schema.into();
        let fixture = Fixture::new(&serde_json::to_vec(&config)?)?;
        check!(matches!(fixture.armed(), Err(BoundaryError::Schema(value)) if value == schema));
    }
    let mut config = configuration(1, 1);
    config["retry"] = true.into();
    let fixture = Fixture::new(&serde_json::to_vec(&config)?)?;
    check!(matches!(fixture.armed(), Err(BoundaryError::Json(_))));
    let bytes = format!("{{\"schema\":1,\"schema\":1,\"operation\":\"{OPERATION}\",\"attempt\":1,\"timeout_seconds\":1}}");
    let fixture = Fixture::new(bytes.as_bytes())?;
    check!(matches!(fixture.armed(), Err(BoundaryError::Json(_))));
    Ok(())
}

#[test]
fn malformed_missing_and_wrongly_typed_fields_never_disable_injection() -> TestResult {
    for bytes in [b"".as_slice(), b"null", b"{}", b"{", b"{\"schema\":1}"] {
        let fixture = Fixture::new(bytes)?;
        check!(matches!(fixture.armed(), Err(BoundaryError::Json(_))));
    }
    for (field, value) in [
        ("attempt", serde_json::json!(256)),
        ("attempt", serde_json::json!(-1)),
        ("attempt", serde_json::json!(1.5)),
        ("timeout_seconds", serde_json::json!(-1)),
        ("timeout_seconds", serde_json::json!("60")),
        ("schema", serde_json::json!(true)),
    ] {
        let mut config = configuration(1, 1);
        config[field] = value;
        let fixture = Fixture::new(&serde_json::to_vec(&config)?)?;
        check!(matches!(fixture.armed(), Err(BoundaryError::Json(_))));
    }
    Ok(())
}

#[test]
fn byte_limit_accepts_4096_and_refuses_4097_without_truncating_json() -> TestResult {
    let mut bytes = serde_json::to_vec(&configuration(1, 1))?;
    bytes.resize(4096, b' ');
    let fixture = Fixture::new(&bytes)?;
    check!(eq; fixture.armed()?.config.select(OPERATION, 1),
    Selection::Hold(Duration::from_secs(1)));
    bytes.push(b' ');
    let fixture = Fixture::new(&bytes)?;
    check!(matches!(
        fixture.armed(),
        Err(BoundaryError::Artifact { .. })
    ));
    Ok(())
}

#[test]
fn empty_wildcard_whitespace_control_and_oversized_operations_are_refused() -> TestResult {
    for operation in [
        "".to_string(),
        "*".to_string(),
        "teams/?".to_string(),
        "teams/ milesplit".to_string(),
        "teams/\n".to_string(),
        "x".repeat(1025),
    ] {
        let mut config = configuration(1, 1);
        config["operation"] = operation.into();
        let fixture = Fixture::new(&serde_json::to_vec(&config)?)?;
        check!(matches!(fixture.armed(), Err(BoundaryError::Operation)));
    }
    let mut config = configuration(1, 1);
    config["operation"] = "x".repeat(1024).into();
    let fixture = Fixture::new(&serde_json::to_vec(&config)?)?;
    check!(eq; fixture.armed()?.config.select(&"x".repeat(1024), 1),
    Selection::Hold(Duration::from_secs(1)));
    Ok(())
}

#[test]
fn config_symlinks_nonregular_files_and_public_directories_are_refused() -> TestResult {
    let fixture = fixture(1, 1)?;
    let alias = fixture.root.path().join("alias.json");
    symlink(&fixture.config, &alias)?;
    check!(matches!(
        super::super::read_armed(&alias),
        Err(BoundaryError::Artifact { .. })
    ));
    let nonregular = fixture.root.path().join("directory.json");
    std::fs::create_dir(&nonregular)?;
    check!(matches!(
        super::super::read_armed(&nonregular),
        Err(BoundaryError::Artifact { .. })
    ));
    std::fs::set_permissions(fixture.root.path(), Permissions::from_mode(0o755))?;
    check!(matches!(
        fixture.armed(),
        Err(BoundaryError::Artifact { .. })
    ));
    Ok(())
}

#[test]
fn symlinked_parent_missing_config_empty_path_and_writable_config_fail_closed() -> TestResult {
    let fixture = fixture(1, 1)?;
    let outer = tempfile::tempdir()?;
    let alias = outer.path().join("directory-alias");
    symlink(fixture.root.path(), &alias)?;
    check!(matches!(
        super::super::read_armed(&alias.join("native-source-boundary-config.json")),
        Err(BoundaryError::Artifact { .. })
    ));
    check!(matches!(
        super::super::read_armed(&fixture.root.path().join("missing.json")),
        Err(BoundaryError::Artifact { .. })
    ));
    check!(matches!(
        super::super::read_armed(std::path::Path::new("")),
        Err(BoundaryError::Path(_))
    ));
    std::fs::set_permissions(&fixture.config, Permissions::from_mode(0o666))?;
    check!(matches!(
        fixture.armed(),
        Err(BoundaryError::Artifact { .. })
    ));
    Ok(())
}
