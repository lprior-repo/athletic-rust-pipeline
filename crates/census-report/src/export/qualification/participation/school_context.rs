use super::archive::SchoolClosure;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::ReplayedResultSchoolCorrespondencesV1;

pub(crate) struct SchoolContext<'a, 'source> {
    pub(crate) source: &'a ReplayedResultSchoolCorrespondencesV1<'source>,
    pub(crate) closure: &'a SchoolClosure,
    pub(crate) grant: &'a mut AttachmentReplayGrantV1,
}

type LiveCollection = (
    super::attached::Collected,
    super::archive::attachment::AttachmentClosureDescriptor,
    Option<super::archive::SchoolClosureDescriptor>,
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
                context.grant,
            )?;
            let (raw, reference) = context.source.retained_attachment_closure(context.grant)?;
            let attachment =
                super::archive::capture_attachment(raw, reference, directory, context.grant)?;
            let descriptor = context.closure.install(directory, context.grant)?;
            Ok((rows, attachment, Some(descriptor)))
        }
        None => {
            let (rows, attachment) =
                super::attached::capture(dataset, snapshot, profile, captured_at, directory)?;
            Ok((rows, attachment, None))
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
    use super::{archive, attached, ParticipationReplay};
    match replay {
        ParticipationReplay::V3(value) => {
            attached::replay(dataset, profile, captured_at, &value.attachment, directory)
        }
        ParticipationReplay::V4(value) => match &value.school {
            Some(descriptor) => {
                let mut grant = AttachmentReplayGrantV1::new();
                let closure = archive::load_school(descriptor, directory, &mut grant)?;
                let school = closure.replay(
                    &dataset.lineage.store_identity,
                    dataset.lineage.snapshot_sequence,
                    &mut grant,
                )?;
                attached::school_bound(dataset, profile, captured_at, &school, &mut grant)
            }
            None => attached::replay_current(
                dataset,
                profile,
                captured_at,
                &value.attachment,
                directory,
            ),
        },
        ParticipationReplay::V1(_) | ParticipationReplay::V2(_) => Ok(attached::Collected {
            rows: std::collections::BTreeMap::new(),
            mapped: std::collections::BTreeMap::new(),
            unsupported: Vec::new(),
            documentary_authority: None,
        }),
    }
}
