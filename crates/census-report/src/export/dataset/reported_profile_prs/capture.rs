use super::{invalid, replay, ReportedProfilePrs, MAX_ROOTS};
use crate::export::dataset::proof_objects;
use crate::export::dataset::selected_proofs::objects;
use crate::report::ReportResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalAthlete, ProfileDate, SourceNamespace};
use census_store::{ReadBudget, RetainedCaptureRootSchemaV1, StoreError, StoreSnapshot};
use std::collections::BTreeSet;
use std::path::Path;

struct Selection<'a> {
    subjects: BTreeSet<&'a str>,
    as_of: &'a ProfileDate,
    directory: &'a Path,
    bytes: u64,
}

impl ReportedProfilePrs {
    pub(in crate::export::dataset) fn extend_selected(
        loaded: &mut crate::export::dataset::Loaded,
        snapshot: &StoreSnapshot<'_>,
        profile: &census_domain::model::QueryProfileV1,
        directory: &Path,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<()> {
        if profile.input().include_provisional
            == census_domain::model::ProvisionalInclusionV1::Include
        {
            loaded.reported_profile_prs = Self::capture(
                snapshot,
                &loaded.athletes,
                &profile.input().as_of,
                directory,
                grant,
            )?;
        }
        Ok(())
    }

    pub(in crate::export::dataset) fn capture(
        snapshot: &StoreSnapshot<'_>,
        athletes: &[CanonicalAthlete],
        as_of: &ProfileDate,
        directory: &Path,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<Self> {
        objects::require_empty(directory)?;
        let selected = athletes
            .iter()
            .flat_map(CanonicalAthlete::identities)
            .filter(|source| source.namespace == SourceNamespace::MilesplitAthlete)
            .map(|source| source.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut reported = Self::default();
        let mut selection = Selection {
            subjects: selected,
            as_of,
            directory,
            bytes: 0,
        };
        snapshot.for_each_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
            ReadBudget {
                rows: 25_000,
                bytes: 64 * 1024 * 1024,
                record_bytes: census_store::MAX_JOURNAL_VALUE_BYTES,
            },
            grant,
            |held| {
                let result = reported.capture_root(
                    held.root(),
                    held.payload().get().as_bytes(),
                    &mut selection,
                );
                result.map_err(|error| StoreError::Refused {
                    detail: error.to_string(),
                })
            },
        )?;
        reported.validate_roots()?;
        Ok(reported)
    }

    fn capture_root(
        &mut self,
        root: &census_store::RetainedCaptureRootV1,
        bytes: &[u8],
        selection: &mut Selection<'_>,
    ) -> ReportResult<()> {
        let (packet, source) = replay::packet(root, bytes)?;
        if !selection.subjects.contains(source.id.as_str())
            || packet
                .profile_capture()
                .acquired_at()
                .date_naive()
                .to_string()
                .as_str()
                > selection.as_of.as_str()
        {
            return Ok(());
        }
        if self.roots.len() >= MAX_ROOTS {
            return Err(invalid("reported profile PR root count exceeds its bound"));
        }
        let total = selection
            .bytes
            .checked_add(root.payload.bytes)
            .ok_or_else(|| invalid("reported profile PR byte count overflow"))?;
        if total > objects::MAX_BYTES {
            return Err(invalid(
                "reported profile PR sources exceed the archive limit",
            ));
        }
        let subject = source.id.clone();
        let table = replay::table(root, &packet, source)?;
        self.roots
            .try_reserve(1)
            .map_err(|_| invalid("profile PR root allocation refused"))?;
        self.roots.push(root.clone());
        if proof_objects::install(selection.directory, bytes)? != root.payload.digest.as_str() {
            return Err(invalid(
                "reported profile PR source blob changed during capture",
            ));
        }
        selection.bytes = total;
        self.tables.entry(subject).or_default().push(table);
        Ok(())
    }
}
