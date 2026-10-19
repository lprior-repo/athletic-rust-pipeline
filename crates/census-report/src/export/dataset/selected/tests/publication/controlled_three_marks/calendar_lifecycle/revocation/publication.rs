mod support;
use super::super::super::flow;
use super::super::super::point_source;
use super::super::super::selection;
use super::super::super::source::{install_with_result_venue, InstalledSource, TestResult};
#[test]
fn calendar_venue_mismatch_requires_bridge_then_revokes_to_next() -> TestResult {
    let installed = install_with_result_venue("Undocumented Track Venue")?;
    let (current, _) = flow::reviewed_inputs(&installed)?;
    let profile = selection::profile()?;
    installed.store.register_profile(&profile)?;
    let points = point_source::install(&installed)?;
    let early = points.get(0).ok_or("Feb28 source missing")?;
    let fifty = points.get(1).ok_or("May15 source missing")?;
    let fiftyone = points.get(2).ok_or("May16 source missing")?;
    let proposal49 = support::propose_for(&installed, early, 0)?;
    let proposal50 = support::propose_for(&installed, fifty, 1)?;
    let proposal51 = support::propose_for(&installed, fiftyone, 2)?;
    support::require_withholds(&installed, &profile)?;
    let (accept50, receipt50) =
        support::bridge_all(&installed, &proposal49, &proposal50, &proposal51)?;
    let public = installed.public.clone();
    let historical = installed.school.clone();
    let output0 = tempfile::tempdir()?;
    let (lineage0, original, workbook0) = support::publish_selected(
        &installed,
        &profile,
        &current,
        &public,
        &historical,
        false,
        &output0.path().join("publication"),
    )?;
    support::revoke_only_50(&installed, &proposal50, accept50, receipt50)?;
    let output1 = tempfile::tempdir()?;
    let (lineage1, withdrawn, workbook1) = support::publish_selected(
        &installed,
        &profile,
        &current,
        &public,
        &historical,
        true,
        &output1.path().join("publication"),
    )?;
    check!(lineage1.snapshot_sequence > lineage0.snapshot_sequence);
    check!(withdrawn.as_str() != original.as_str());
    super::super::super::source::require_originals(&installed.store, &installed.source_records)?;
    let InstalledSource {
        directory, store, ..
    } = installed;
    drop(store);
    directory.close()?;
    support::reopen(
        &workbook0,
        &lineage0,
        &public,
        &current,
        &historical,
        original.as_str(),
        false,
    )?;
    support::reopen(
        &workbook1,
        &lineage1,
        &public,
        &current,
        &historical,
        withdrawn.as_str(),
        true,
    )?;
    flow::retain_outputs(output0, output1)
}
