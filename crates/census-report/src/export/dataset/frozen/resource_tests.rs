use super::codec::FrozenCodecError;
use super::fixtures::{school, TestResult};
use super::reader::{EncodedWindowLimit, TableCursor};
use super::stream::StreamKind;
use super::{codec_error, freeze_v2, objects, verify};
use crate::report::{CorruptionClass, ExportResourceError, ReportError};
use census_domain::model::CanonicalSchool;
use census_store::{Store, Table};

#[test]
fn export_resource_codec_mapping_preserves_each_variant_and_exact_fields() -> TestResult {
    let cases = [
        (
            FrozenCodecError::RowTooLarge {
                what: "frozen line",
                bytes: 1_048_577,
                max: 1_048_576,
            },
            ExportResourceError::RowTooLarge {
                what: "frozen line",
                bytes: 1_048_577,
                max: 1_048_576,
            },
        ),
        (
            FrozenCodecError::BudgetExceeded {
                what: "frozen json depth",
                needed: 65,
                available: 64,
            },
            ExportResourceError::BudgetExceeded {
                what: "frozen json depth",
                needed: 65,
                available: 64,
            },
        ),
        (
            FrozenCodecError::NeedsPartition {
                what: "frozen inventory",
                bytes: 4_194_305,
                max: 4_194_304,
            },
            ExportResourceError::NeedsPartition {
                what: "frozen inventory",
                bytes: 4_194_305,
                max: 4_194_304,
            },
        ),
    ];
    for (codec, expected) in cases {
        let error = codec_error(codec);
        check!(eq; error.resource(), Some(&expected));
        check!(eq; error.corruption(), None);
        check!(matches!(error, ReportError::Resource(actual) if actual == expected));
    }
    Ok(())
}

#[test]
fn export_resource_nonresource_codec_errors_keep_the_existing_corruption_class() -> TestResult {
    let cases = [
        FrozenCodecError::CorruptArtifact {
            what: "frozen line",
            detail: "wrong ordinal".into(),
        },
        FrozenCodecError::UnsupportedVersion {
            what: "frozen manifest",
            found: "unknown".into(),
            supported: "current".into(),
        },
        FrozenCodecError::Encoding {
            detail: "encoding failed".into(),
        },
    ];
    for codec in cases {
        let expected = format!("frozen codec: {codec}");
        let error = codec_error(codec);
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(eq; error.resource(), None);
        check!(
            matches!(error, ReportError::ArtifactCorrupt { path, detail } if path.as_os_str().is_empty() && detail == expected)
        );
    }
    Ok(())
}

#[test]
fn export_resource_cursor_refusal_keeps_its_root_and_visits_no_disallowed_row() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(Table::Schools, &school("Cursor Resource High"))?;
    let root = freeze_v2(&store, "export-resource:cursor")?;
    let before = store
        .get_input_record(root.attempt())?
        .ok_or("missing input record")?;
    let object_dir = objects::directory(root.directory());
    let manifest_before = std::fs::read(object_dir.join(root.manifest_digest()))?;
    let reference = root
        .manifest()
        .stream(StreamKind::Schools)
        .ok_or("school stream missing")?;
    let page = verify::page(&object_dir, reference)?;
    let cursor = TableCursor::open(&object_dir, StreamKind::Schools, &page)?;
    let mut visited = 0_u64;
    let result = cursor.visit::<CanonicalSchool>(EncodedWindowLimit::new(1, 1)?, |_| {
        visited = visited
            .checked_add(1)
            .ok_or(FrozenCodecError::BudgetExceeded {
                what: "fixture counter",
                needed: u64::MAX,
                available: 0,
            })?;
        Ok(())
    });
    let error = match result {
        Ok(_) => return Err("out-of-budget cursor was accepted".into()),
        Err(error) => error,
    };
    check!(
        matches!(error, ReportError::Resource(ExportResourceError::BudgetExceeded {
        what: "frozen window encoded bytes", available: 1, needed,
    }) if needed > 1)
    );
    check!(eq; visited, 0);
    check!(eq; std::fs::read(object_dir.join(root.manifest_digest()))?, manifest_before);
    check!(eq; store.get_input_record(root.attempt())?.ok_or("missing input record")?, before);
    Ok(())
}
