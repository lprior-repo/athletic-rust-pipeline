use super::*;
use census_crawl::participation_evidence::recorded::read_recorded_provider_school;
mod agreement;
mod retained;

pub(super) fn read(
    installed: &source::Installed,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<(AttachmentDigestV1, AttachmentDigestV1, AttachmentDigestV1)> {
    let snapshot = installed.store.snapshot();
    let provider = read_recorded_provider_school(&snapshot, &installed.school_reference, grant)?
        .ok_or("actual retained MS target roster")?;
    let participation =
        read_recorded_independent_participation(&snapshot, &installed.participation, grant)?
            .ok_or("independent Roncalli exact-day person context")?;
    let result = read_recorded_tfrrs_list_capture(
        &snapshot,
        &installed.context,
        &[source::locator()],
        grant,
    )?
    .ok_or("native TF context retained unchanged")?;
    let checked_result = result.read(&[source::locator()])?;
    let row = checked_result.row(0).ok_or("exact native TF result")?;
    check!(eq; provider.checked().school(),&installed.school);
    let roster = provider
        .checked()
        .roster_verdict()
        .roster()
        .ok_or("actual parsed MS roster")?;
    check!(eq; roster.athletes.len(),195);
    let mut matches = roster
        .athletes
        .iter()
        .filter(|row| row.athlete().athlete_id == "13658157");
    let target = matches.next().ok_or("literal native MS target row")?;
    check!(matches.next().is_none());
    agreement::verify(installed, target, participation.checked(), &row)?;
    let locator = target.locator();
    let end = locator
        .byte_offset
        .checked_add(locator.byte_length)
        .ok_or("MS row locator overflow")?;
    let original = captures::ROSTER
        .get(locator.byte_offset..end)
        .ok_or("literal MS row bytes")?;
    let fact = objects.bytes(original.to_vec())?;
    let lineage = objects.json(&(
        "genuine-tessa-publisher/v1",
        "milesplit.com",
        provider.root(),
    ))?;
    let roster_capture = retained::capture(
        &snapshot,
        objects,
        provider.root(),
        "roster",
        RetainedCaptureSlotV1::ProviderSchoolRoster,
        captures::ROSTER,
    )?;
    let status = retained::capture(
        &snapshot,
        objects,
        participation.root(),
        "status",
        RetainedCaptureSlotV1::ParticipationStatus,
        roncalli_fixture::REVIEW.as_bytes(),
    )?;
    let calendar = retained::capture(
        &snapshot,
        objects,
        participation.root(),
        "schedule",
        RetainedCaptureSlotV1::ParticipationSchedule,
        roncalli_fixture::SCHEDULE.as_bytes(),
    )?;
    let context = agreement::context(
        objects,
        target,
        participation.checked(),
        &row,
        (&roster_capture, &status, &calendar),
    )?;
    Ok((fact, lineage, context))
}
