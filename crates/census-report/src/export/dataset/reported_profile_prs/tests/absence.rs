use super::{source, ReportedProfilePrs, TestResult};
use crate::export::dataset::initial_objects::InitialObjects;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalAthlete, Gender, GradYear, ProfileDate, SchoolId, SourceIdentity, SourceNamespace,
};
use census_store::{RetainedCaptureRootSchemaV1, Store};
use std::path::Path;

fn capture_for(
    store: &Store,
    id: &str,
    day: &str,
    directory: &Path,
) -> TestResult<ReportedProfilePrs> {
    let athlete = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["source PR control"]),
        "Source PR control",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, id),
    )?;
    Ok(ReportedProfilePrs::capture(
        &store.snapshot(),
        &[athlete],
        &ProfileDate::parse(day)?,
        directory,
        &mut AttachmentReplayGrantV1::new(),
        &InitialObjects::fresh(),
    )?)
}

fn without_pr_table(html: String) -> TestResult<String> {
    let start = html
        .find("<table id=\"prTable\"")
        .ok_or("fixture PR table")?;
    let tail = html.get(start..).ok_or("fixture table start")?;
    let length = tail
        .find("</table>")
        .and_then(|n| n.checked_add("</table>".len()))
        .ok_or("fixture table end")?;
    let end = start.checked_add(length).ok_or("fixture table span")?;
    Ok(format!(
        "{}{}",
        html.get(..start).ok_or("fixture prefix")?,
        html.get(end..).ok_or("fixture suffix")?
    ))
}

#[test]
fn selected_valid_target_without_a_pr_table_retains_unknown_and_cold_replays() -> TestResult {
    let directory = tempfile::tempdir()?;
    let objects = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let packet = source::packet_with_profile("7223777", "2026-10-16T00:50:27Z", without_pr_table)?;
    check!(packet.read_profile_prs_if_published()?.is_none());
    check!(packet.read_profile_prs().is_err());
    store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    let reported = capture_for(&store, "7223777", "2026-10-21", objects.path())?;
    check!(eq; reported.roots.len(), 1);
    check!(reported
        .tables
        .values()
        .flatten()
        .all(|table| table.rows.is_empty()));
    reported.require_initial_objects(objects.path(), &InitialObjects::fresh())?;
    let root = reported.roots.first().ok_or("retained profile root")?;
    let bytes = crate::export::proof_objects::read(
        objects.path(),
        root.payload.digest.as_str(),
        root.payload.bytes,
    )?;
    drop(store);
    directory.close()?;
    let (subject, table) = super::super::replay::read(root, &bytes)?;
    check!(eq; subject, "7223777");
    check!(table.rows.is_empty());
    Ok(())
}

#[test]
fn unrelated_and_future_captures_are_filtered_before_optional_pr_parsing() -> TestResult {
    let transforms: [fn(String) -> TestResult<String>; 2] = [without_pr_table, |html| {
        Ok(html.replacen("id=\"prTable\"", "id=\"wrongTable\"", 1))
    }];
    for transform in transforms {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let packet = source::packet_with_profile("7223777", "2026-10-16T00:50:27Z", transform)?;
        store.journal_done(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            packet.reference()?.as_str(),
            &packet,
        )?;
        for (subject, day) in [("unrelated", "2026-10-21"), ("7223777", "2026-10-15")] {
            let objects = tempfile::tempdir()?;
            let reported = capture_for(&store, subject, day, objects.path())?;
            check!(reported.is_empty());
            reported.require_initial_objects(objects.path(), &InitialObjects::fresh())?;
        }
    }
    Ok(())
}

#[test]
fn a_malformed_present_pr_table_for_a_selected_subject_refuses() -> TestResult {
    let directory = tempfile::tempdir()?;
    let objects = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let packet = source::packet_with_profile("7223777", "2026-10-16T00:50:27Z", |html| {
        Ok(html.replacen("id=\"prTable\"", "id=\"wrongTable\"", 1))
    })?;
    check!(packet.read_profile_prs_if_published().is_err());
    store.journal_done(
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        packet.reference()?.as_str(),
        &packet,
    )?;
    check!(capture_for(&store, "7223777", "2026-10-21", objects.path()).is_err());
    Ok(())
}
