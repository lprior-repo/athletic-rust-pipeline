use super::*;
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom, Write};

#[derive(Serialize)]
struct RootsWire<'a> {
    roots: &'a [ReportedProfileRootV1],
}

#[test]
fn population_sized_profile_roots_admit_exact_limit_and_refuse_one_more() -> TestResult {
    check!(eq; MAX_ROOTS, 4_000_000);
    let mut file = tempfile::NamedTempFile::new()?;
    {
        let mut roots = Vec::new();
        roots.try_reserve_exact(MAX_ROOTS)?;
        for index in 0..MAX_ROOTS {
            roots.push(root(index)?);
        }
        let reported = ReportedProfilePrs {
            roots,
            ..Default::default()
        };
        reported.validate_roots()?;
        write_roots(file.as_file_mut(), &reported)?;
    }
    let mut decoded: ReportedProfilePrs = serde_json::from_reader(BufReader::with_capacity(
        64 * 1024,
        File::open(file.path())?,
    ))?;
    check!(eq; decoded.roots.len(), MAX_ROOTS);
    decoded.validate_roots()?;
    decoded.roots.try_reserve_exact(1)?;
    decoded.roots.push(root(MAX_ROOTS)?);
    check!(decoded.validate_roots().is_err());
    write_roots(file.as_file_mut(), &decoded)?;
    drop(decoded);
    let extra = serde_json::from_reader::<_, ReportedProfilePrs>(BufReader::with_capacity(
        64 * 1024,
        File::open(file.path())?,
    ));
    check!(extra.is_err());
    Ok(())
}

fn write_roots(file: &mut File, reported: &ReportedProfilePrs) -> TestResult {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    serde_json::to_writer(
        &mut writer,
        &RootsWire {
            roots: &reported.roots,
        },
    )?;
    writer.flush()?;
    Ok(())
}

#[test]
fn owned_profile_key_must_match_its_exact_payload_digest() -> TestResult {
    let mut reported = ReportedProfilePrs {
        roots: vec![root(0)?],
        ..Default::default()
    };
    reported.validate_roots()?;
    let first = reported.roots.first_mut().ok_or("profile root")?;
    first.key = format!("{:064x}", 1);
    check!(reported.validate_roots().is_err());
    Ok(())
}

fn root(index: usize) -> TestResult<ReportedProfileRootV1> {
    let key = format!("{index:064x}");
    Ok(ReportedProfileRootV1 {
        schema: ReportedProfileRootSchemaV1::OwnedPublicProfileCaptureV1,
        payload: AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(&key)?,
            bytes: 1,
        },
        key,
    })
}
