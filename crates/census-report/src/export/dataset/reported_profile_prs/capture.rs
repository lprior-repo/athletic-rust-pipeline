use super::{invalid, replay, ReportedProfilePrs, ReportedProfileRootV1, MAX_ROOTS};
use crate::export::dataset::initial_objects::InitialObjects;
use crate::export::dataset::proof_objects;
use crate::export::dataset::selected_proofs::objects;
use crate::report::ReportResult;
use census_crawl::milesplit::PROFILE_PR_PHASE;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalAthlete, ProfileDate, SourceNamespace};
use census_store::read::JournalNamespace;
use census_store::{
    ReadBudget, RetainedCaptureRootSchemaV1, StoreError, StoreResult, StoreSnapshot,
};
use sha2::{Digest, Sha256};
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
        initial: &InitialObjects,
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
                initial,
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
        initial: &InitialObjects,
    ) -> ReportResult<Self> {
        initial.require_initial(directory)?;
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
        for ordinal in 0_u16..256 {
            scan_prefix(snapshot, &shard(ordinal), &mut reported, &mut selection)?;
        }
        reported.roots.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        reported.validate_roots()?;
        Ok(reported)
    }
}

fn shard(ordinal: u16) -> String {
    format!("{ordinal:02x}")
}

fn scan_prefix(
    snapshot: &StoreSnapshot<'_>,
    prefix: &str,
    reported: &mut ReportedProfilePrs,
    selection: &mut Selection<'_>,
) -> ReportResult<()> {
    snapshot.for_each_journal_root_bounded(
        JournalNamespace::new(PROFILE_PR_PHASE, prefix)?,
        ReadBudget {
            rows: 25_000,
            bytes: 64 * 1024 * 1024,
            record_bytes: census_store::MAX_JOURNAL_VALUE_BYTES,
        },
        |key| visit_journal_row(snapshot, reported, selection, key),
    )?;
    Ok(())
}

fn visit_journal_row(
    snapshot: &StoreSnapshot<'_>,
    reported: &mut ReportedProfilePrs,
    selection: &mut Selection<'_>,
    key: &str,
) -> StoreResult<()> {
    let payload = snapshot
        .journal_payload(PROFILE_PR_PHASE, key)?
        .ok_or_else(|| StoreError::Refused {
            detail: format!("profile PR journal key {key} has no payload"),
        })?;
    let bytes = serde_json::to_vec(&payload).map_err(|error| StoreError::Refused {
        detail: error.to_string(),
    })?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let count = u64::try_from(bytes.len()).map_err(|_| StoreError::Refused {
        detail: "profile PR journal row byte count".to_string(),
    })?;
    let root =
        ReportedProfileRootV1::journal(&digest, count).map_err(|error| StoreError::Refused {
            detail: error.to_string(),
        })?;
    reported
        .capture_root(&root, &bytes, selection)
        .map_err(|error| StoreError::Refused {
            detail: error.to_string(),
        })
}

impl ReportedProfilePrs {
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
