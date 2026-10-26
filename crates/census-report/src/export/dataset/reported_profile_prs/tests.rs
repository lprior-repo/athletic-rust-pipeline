use super::*;
use crate::export::dataset::initial_objects::InitialObjects;

mod absence;
mod bounds;
mod journal;
pub(crate) mod source;
mod subjects;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn empty_collection_has_no_source_claims_or_frozen_roots() -> TestResult {
    let empty = ReportedProfilePrs::default();
    check!(empty.is_empty());
    empty.validate_roots()?;
    check!(eq; serde_json::to_string(&empty)?, r#"{"roots":[]}"#);
    Ok(())
}

#[test]
fn retained_genuine_prs_freeze_and_reparse_without_accepting_a_canonical_mark() -> TestResult {
    use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
    use census_domain::model::{
        CanonicalAthlete, Gender, GradYear, ProfileDate, SchoolId, SourceIdentity, SourceNamespace,
    };
    use census_store::{RetainedCaptureRootSchemaV1, Store, Table};
    let directory = tempfile::tempdir()?;
    let objects = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let packet = source::packet("7223777", "2026-10-16T00:50:27Z")?;
    store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    let athlete = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["source PR control"]),
        "Source PR control",
        GradYear::new(2026).ok_or("year")?,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "7223777"),
    )?;
    let before = store
        .snapshot()
        .tables_digest(&[Table::Athletes, Table::Performances])?;
    let reported = ReportedProfilePrs::capture(
        &store.snapshot(),
        &[athlete],
        &ProfileDate::parse("2026-10-21")?,
        objects.path(),
        &mut AttachmentReplayGrantV1::new(),
        &InitialObjects::fresh(),
    )?;
    reported.require_initial_objects(objects.path(), &InitialObjects::fresh())?;
    check!(eq; reported.roots.len(), 1);
    let serialized = serde_json::to_vec(&reported)?;
    let reopened: ReportedProfilePrs = serde_json::from_slice(&serialized)?;
    let root = reopened.roots.first().ok_or("retained PR root")?;
    let bytes = crate::export::proof_objects::read(
        objects.path(),
        root.payload.digest.as_str(),
        root.payload.bytes,
    )?;
    check!(eq; store.snapshot().tables_digest(&[Table::Athletes, Table::Performances])?, before);
    drop(store);
    directory.close()?;
    let (subject, table) = replay::read(root, &bytes)?;
    check!(eq; subject, "7223777");
    check!(eq; table.rows.len(), 46);
    check!(table
        .rows
        .iter()
        .any(|r| r.source_mode() == "indoor" && r.source_level() == "hs"));
    check!(table
        .rows
        .iter()
        .any(|r| r.source_mode() == "outdoor" && r.source_level() == "ms"));
    check!(table
        .rows
        .iter()
        .any(|r| r.source_mode() == "all" && r.source_level() == "all"));
    let file = objects.path().join(root.payload.digest.as_str());
    std::fs::write(&file, b"controlled corruption")?;
    check!(reopened
        .require_initial_objects(objects.path(), &InitialObjects::fresh())
        .is_err());
    check!(replay::read(root, b"controlled corruption").is_err());
    std::fs::remove_file(&file)?;
    check!(reopened
        .require_initial_objects(objects.path(), &InitialObjects::fresh())
        .is_err());
    Ok(())
}

#[test]
fn captured_prs_respect_exact_source_identity_and_capture_cutoff() -> TestResult {
    use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
    use census_domain::model::{
        CanonicalAthlete, Gender, GradYear, ProfileDate, SchoolId, SourceIdentity, SourceNamespace,
    };
    use census_store::{RetainedCaptureRootSchemaV1, Store};
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let packet = source::packet("7223777", "2026-10-16T00:50:27Z")?;
    store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    for (id, day) in [
        ("different-person", "2026-10-21"),
        ("7223777", "2026-10-15"),
    ] {
        let athlete = CanonicalAthlete::new(
            &SchoolId::mint("sch", &["source PR control"]),
            "Source PR control",
            GradYear::CO2027,
            Gender::Boys,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, id),
        )?;
        let objects = tempfile::tempdir()?;
        let reported = ReportedProfilePrs::capture(
            &store.snapshot(),
            &[athlete],
            &ProfileDate::parse(day)?,
            objects.path(),
            &mut AttachmentReplayGrantV1::new(),
            &InitialObjects::fresh(),
        )?;
        check!(reported.is_empty());
        reported.require_initial_objects(objects.path(), &InitialObjects::fresh())?;
    }
    Ok(())
}
