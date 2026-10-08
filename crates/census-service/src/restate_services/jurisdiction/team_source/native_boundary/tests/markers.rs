use std::os::unix::fs::{symlink, MetadataExt};

use serde_json::json;

use super::{expected_marker, fixture, reserved_marker, write_new, BoundaryError, TestResult};

#[test]
fn exact_existing_authority_is_retained_even_while_it_has_another_hard_link() -> TestResult {
    let fixture = fixture(1, 60)?;
    let armed = fixture.armed()?;
    let bytes = serde_json::to_vec(&expected_marker(1))?;
    write_new(&fixture.marker(), &bytes)?;
    let retained_link = fixture.root.path().join("retained-marker-link.json");
    std::fs::hard_link(fixture.marker(), &retained_link)?;
    reserved_marker(1)?.publish(&armed.directory)?;
    check!(eq; std::fs::read(fixture.marker())?, bytes);
    check!(eq; std::fs::read(retained_link)?, bytes);
    check!(eq; std::fs::symlink_metadata(fixture.pending())
        .err()
        .map(|error| error.kind()),
    Some(std::io::ErrorKind::NotFound));
    Ok(())
}

#[test]
fn each_marker_identity_mismatch_is_refused_without_overwriting_authority() -> TestResult {
    for (field, value) in [
        ("schema", json!(2)),
        ("phase", json!("physical_http")),
        ("operation", json!("jurisdiction:RI:2026-27:1/teams/riil")),
        ("attempt", json!(2)),
        ("request_digest", json!("different-journaled-request")),
        ("observed_on", json!("2026-10-03")),
    ] {
        let fixture = fixture(1, 60)?;
        let armed = fixture.armed()?;
        let mut authority = expected_marker(1);
        authority[field] = value;
        let bytes = serde_json::to_vec(&authority)?;
        write_new(&fixture.marker(), &bytes)?;
        check!(matches!(
            reserved_marker(1)?.publish(&armed.directory),
            Err(BoundaryError::IdentityMismatch)
        ));
        check!(eq; std::fs::read(fixture.marker())?, bytes);
        check!(eq; std::fs::symlink_metadata(fixture.pending())
            .err()
            .map(|error| error.kind()),
        Some(std::io::ErrorKind::NotFound));
    }
    Ok(())
}

#[test]
fn unknown_fields_or_incomplete_existing_authority_are_not_trusted() -> TestResult {
    let mut extra = expected_marker(1);
    extra["physical_http_started"] = json!(true);
    let mut missing = expected_marker(1);
    let _removed = missing
        .as_object_mut()
        .ok_or("expected marker object")?
        .remove("observed_on")
        .ok_or("missing original observation date")?;
    for value in [extra, missing] {
        let fixture = fixture(1, 60)?;
        let armed = fixture.armed()?;
        let bytes = serde_json::to_vec(&value)?;
        write_new(&fixture.marker(), &bytes)?;
        check!(matches!(
            reserved_marker(1)?.publish(&armed.directory),
            Err(BoundaryError::Json(_))
        ));
        check!(eq; std::fs::read(fixture.marker())?, bytes);
    }
    Ok(())
}

#[test]
fn oversized_authority_is_refused_before_decode_without_replacing_its_bytes() -> TestResult {
    let fixture = fixture(1, 60)?;
    let armed = fixture.armed()?;
    let prefix = serde_json::to_vec(&expected_marker(1))?;
    write_new(&fixture.marker(), &prefix)?;
    let length = u64::try_from(super::super::files::MAX_MARKER_BYTES)?
        .checked_add(1)
        .ok_or("native marker boundary length overflow")?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(fixture.marker())?;
    file.set_len(length)?;
    file.sync_all()?;
    let before = std::fs::metadata(fixture.marker())?;
    check!(matches!(
        reserved_marker(1)?.publish(&armed.directory),
        Err(BoundaryError::Artifact { .. })
    ));
    let after = std::fs::metadata(fixture.marker())?;
    check!(eq; after.len(), length);
    check!(eq; after.ino(), before.ino());
    let mut held = vec![0; prefix.len()];
    std::io::Read::read_exact(&mut std::fs::File::open(fixture.marker())?, &mut held)?;
    check!(eq; held, prefix);
    Ok(())
}

#[test]
fn preexisting_pending_file_is_refused_and_preserved_even_with_matching_authority() -> TestResult {
    let fixture = fixture(1, 60)?;
    let armed = fixture.armed()?;
    let bytes = serde_json::to_vec(&expected_marker(1))?;
    write_new(&fixture.marker(), &bytes)?;
    write_new(&fixture.pending(), b"belongs to another worker")?;
    check!(matches!(
        reserved_marker(1)?.publish(&armed.directory),
        Err(BoundaryError::Artifact { .. })
    ));
    check!(eq; std::fs::read(fixture.pending())?,
    b"belongs to another worker");
    check!(eq; std::fs::read(fixture.marker())?, bytes);
    Ok(())
}

#[test]
fn symlink_and_nonregular_marker_or_pending_names_never_change_their_targets() -> TestResult {
    let fixture = fixture(1, 60)?;
    let armed = fixture.armed()?;
    let foreign = fixture.root.path().join("foreign.json");
    write_new(&foreign, b"retained foreign content")?;
    symlink(&foreign, fixture.marker())?;
    check!(matches!(
        reserved_marker(1)?.publish(&armed.directory),
        Err(BoundaryError::Artifact { .. })
    ));
    check!(eq; std::fs::read(&foreign)?, b"retained foreign content");
    std::fs::remove_file(fixture.marker())?;
    std::fs::create_dir(fixture.marker())?;
    check!(matches!(
        reserved_marker(1)?.publish(&armed.directory),
        Err(BoundaryError::Artifact { .. })
    ));
    std::fs::remove_dir(fixture.marker())?;
    symlink(&foreign, fixture.pending())?;
    check!(matches!(
        reserved_marker(1)?.publish(&armed.directory),
        Err(BoundaryError::Artifact { .. })
    ));
    check!(eq; std::fs::read(&foreign)?, b"retained foreign content");
    Ok(())
}
