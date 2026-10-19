use super::archive::SchoolClosure;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::ReplayedResultSchoolCorrespondencesV1;

pub(crate) struct SchoolContext<'a, 'source> {
    pub(crate) source: &'a ReplayedResultSchoolCorrespondencesV1<'source>,
    pub(crate) closure: &'a SchoolClosure,
    pub(crate) grant: &'a mut AttachmentReplayGrantV1,
    pub(crate) calendar: Option<CalendarContext<'a, 'source>>,
}

pub(crate) struct CalendarContext<'a, 'source> {
    pub(crate) source: &'a census_store::ReplayedResultCalendarCorrespondencesV1<'source>,
    pub(crate) closure: &'a super::archive::CalendarClosure,
}

type LiveCollection = (
    super::attached::Collected,
    super::archive::attachment::AttachmentClosureDescriptor,
    Option<super::archive::SchoolClosureDescriptor>,
    Option<super::archive::CalendarClosureDescriptor>,
);

pub(super) fn collect_live(
    dataset: &crate::export::ExportDataset,
    snapshot: &census_store::StoreSnapshot<'_>,
    profile: &census_domain::model::QueryProfileV1,
    captured_at: chrono::DateTime<chrono::Utc>,
    directory: &std::path::Path,
    school: Option<SchoolContext<'_, '_>>,
) -> crate::report::ReportResult<LiveCollection> {
    match school {
        Some(context) => {
            let rows = super::attached::school_bound(
                dataset,
                profile,
                captured_at,
                context.source,
                context.calendar.as_ref().map(|calendar| calendar.source),
                context.grant,
            )?;
            let (raw, reference) = context.source.retained_attachment_closure(context.grant)?;
            let attachment =
                super::archive::capture_attachment(raw, reference, directory, context.grant)?;
            let descriptor = context.closure.install(directory, context.grant)?;
            let calendar = context
                .calendar
                .map(|calendar| calendar.closure.install(directory, context.grant))
                .transpose()?;
            Ok((rows, attachment, Some(descriptor), calendar))
        }
        None => {
            let (rows, attachment) =
                super::attached::capture(dataset, snapshot, profile, captured_at, directory)?;
            Ok((rows, attachment, None, None))
        }
    }
}

pub(super) fn replay_documentary(
    dataset: &crate::export::ExportDataset,
    profile: &census_domain::model::QueryProfileV1,
    captured_at: chrono::DateTime<chrono::Utc>,
    replay: &super::ParticipationReplay,
    directory: &std::path::Path,
) -> crate::report::ReportResult<super::attached::Collected> {
    use super::{attached, ParticipationReplay};
    match replay {
        ParticipationReplay::V3(value) => {
            attached::replay(dataset, profile, captured_at, &value.attachment, directory)
        }
        ParticipationReplay::V4(value) => match &value.school {
            Some(descriptor) => replay_school_documentary(
                dataset,
                profile,
                captured_at,
                descriptor,
                value.calendar.as_ref(),
                directory,
            ),
            None => {
                if value.calendar.is_some() {
                    return Err(
                        super::invalid("calendar authority lacks school predecessor").into(),
                    );
                }
                attached::replay_current(
                    dataset,
                    profile,
                    captured_at,
                    &value.attachment,
                    directory,
                )
            }
        },
        ParticipationReplay::V1(_) | ParticipationReplay::V2(_) => Ok(attached::Collected {
            rows: std::collections::BTreeMap::new(),
            mapped: std::collections::BTreeMap::new(),
            unsupported: Vec::new(),
            documentary_authority: None,
        }),
    }
}

fn replay_school_documentary(
    dataset: &crate::export::ExportDataset,
    profile: &census_domain::model::QueryProfileV1,
    captured_at: chrono::DateTime<chrono::Utc>,
    descriptor: &super::archive::SchoolClosureDescriptor,
    calendar: Option<&super::archive::CalendarClosureDescriptor>,
    directory: &std::path::Path,
) -> crate::report::ReportResult<super::attached::Collected> {
    let mut grant = AttachmentReplayGrantV1::new();
    let closure = super::archive::load_school(descriptor, directory, &mut grant)?;
    let school = closure.replay(
        &dataset.lineage.store_identity,
        dataset.lineage.snapshot_sequence,
        &mut grant,
    )?;
    let calendar = calendar
        .map(|descriptor| super::archive::load_calendar(descriptor, directory, &mut grant))
        .transpose()?;
    let source = calendar
        .as_ref()
        .map(|closure| {
            closure.replay(
                &dataset.lineage.store_identity,
                dataset.lineage.snapshot_sequence,
                &mut grant,
            )
        })
        .transpose()?;
    super::attached::school_bound(
        dataset,
        profile,
        captured_at,
        &school,
        source.as_ref(),
        &mut grant,
    )
}
