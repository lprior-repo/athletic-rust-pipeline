use super::{invalid, replay, ReportedProfilePrs, ReportedProfileRootV1, MAX_ROOTS};
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
        scope: super::SourceScope<'_>,
        directory: &Path,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<()> {
        if scope.profile.input().include_provisional
            == census_domain::model::ProvisionalInclusionV1::Include
        {
            let as_of = &scope.profile.input().as_of;
            loaded.reported_profile_prs = Self::capture(
                snapshot,
                super::scope::athletes(loaded, scope)?,
                as_of,
                directory,
                grant,
            )?;
        }
        Ok(())
    }

    pub(in crate::export::dataset) fn capture<'a>(
        snapshot: &StoreSnapshot<'_>,
        athletes: impl IntoIterator<Item = &'a CanonicalAthlete>,
        as_of: &ProfileDate,
        directory: &Path,
        grant: &mut AttachmentReplayGrantV1,
    ) -> ReportResult<Self> {
        objects::require_empty(directory)?;
        let selected = athletes
            .into_iter()
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
                let result = ReportedProfileRootV1::retained(held.root()).and_then(|root| {
                    reported.capture_root(&root, held.payload().get().as_bytes(), &mut selection)
                });
                result.map_err(|error| StoreError::Refused {
                    detail: error.to_string(),
                })
            },
        )?;
        snapshot.for_each_public_profile_capture_v1(|packet| {
            let bytes = packet.encode()?;
            let result = ReportedProfileRootV1::owned(&bytes)
                .and_then(|root| reported.capture_root(&root, &bytes, &mut selection));
            result.map_err(|error| StoreError::Refused {
                detail: error.to_string(),
            })
        })?;
        reported.roots.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        reported.validate_roots()?;
        Ok(reported)
    }

    fn capture_root(
        &mut self,
        root: &ReportedProfileRootV1,
        bytes: &[u8],
        selection: &mut Selection<'_>,
    ) -> ReportResult<()> {
        let (subject, acquired_day) = replay::identity(root, bytes)?;
        if !selection.subjects.contains(subject.as_str())
            || acquired_day.as_str() > selection.as_of.as_str()
        {
            return Ok(());
        }
        let (_, table) = replay::read(root, bytes)?;
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
