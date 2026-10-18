use super::current_affiliation::VerifiedRosterContext;
use crate::export::qualification::physical_observations::PhysicalObservationDescriptor;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_crawl::milesplit::{
    CaptureReadBudget, FrozenRetainedJournal, RecordingRetainedJournal, RetainedJournalView,
    RetainedResultSession,
};
use census_domain::affiliation_candidates::{DeclaredFrozenView, ObjectRef};
use census_domain::model::{serialized_digest, PerformanceId, QueryProfileV1};
use census_domain::participation_candidates::{
    participation_policy_v2, participation_policy_v3, participation_policy_v4, CandidateBinding,
    CandidateQuery,
};
use census_store::clock::{Clock, SystemClock};
use census_store::{StoreError, StoreResult, StoreSnapshot};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::path::Path;

mod archive;
mod attached;
pub(crate) use attached::UnsupportedDocumentarySource;
mod budget;
mod closure;
mod context;
mod contexts;
mod independent;
mod note;
mod observations;
mod resolve;
mod support;
mod types;
mod unresolved;
mod validation;

#[cfg(test)]
pub(crate) use archive::ParticipationReplayV3;
pub(crate) use archive::{
    ParticipationReplay, ParticipationReplayV1, ParticipationReplayV2, ParticipationReplayV4,
};
pub(crate) use observations::BUDGET as PHYSICAL_OBSERVATION_BUDGET;
pub(crate) use types::{ParticipationDiagnostic, PinnedParticipation, PinnedParticipationRow};
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

impl PinnedParticipation {
    pub(crate) fn fingerprints(&self) -> BTreeMap<String, String> {
        self.replay.fingerprints()
    }

    pub(crate) fn is_historical_documentary_replay(&self) -> bool {
        matches!(&self.replay, ParticipationReplay::V3(_))
    }

    pub(crate) fn install_replay(&self, directory: &Path) -> ReportResult<String> {
        self.replay.install(directory)
    }

    pub(crate) fn read(
        dataset: &ExportDataset,
        snapshot: &StoreSnapshot<'_>,
        profile: &QueryProfileV1,
        directory: &Path,
        athletes: &PhysicalObservationDescriptor,
        expected_athletes: &str,
        roster_context: VerifiedRosterContext<'_>,
    ) -> ReportResult<Self> {
        validation::live(dataset, snapshot, profile)?;
        roster_context.require_binding(dataset, profile)?;
        let captured_at = DateTime::parse_from_rfc3339(&SystemClock.today_iso8601())
            .map_err(|_| invalid("snapshot qualification clock is not RFC3339"))?
            .with_timezone(&Utc);
        let (closure, observations) =
            observations::capture(dataset, snapshot, directory, athletes, expected_athletes)?;
        let independent = independent::capture(snapshot, &closure)?;
        let recorder = RecordingRetainedJournal::new(snapshot);
        let rows = assess(
            dataset,
            profile,
            &recorder,
            &closure,
            captured_at,
            roster_context,
            Some(&independent),
        )?;
        seal_live(
            dataset,
            snapshot,
            profile,
            captured_at,
            observations,
            recorder,
            independent,
            rows,
            directory,
        )
    }

    pub(crate) fn replay(
        dataset: &ExportDataset,
        profile: &QueryProfileV1,
        directory: &Path,
        anchored_descriptor: &str,
        expected_fingerprints: &BTreeMap<String, String>,
        roster_context: VerifiedRosterContext<'_>,
    ) -> ReportResult<Self> {
        let replay = ParticipationReplay::load(directory, anchored_descriptor)?;
        validation::frozen(dataset, profile, &replay)?;
        roster_context.require_binding(dataset, profile)?;
        let retained = replay.retained();
        let closure = observations::read(
            dataset,
            directory,
            &retained.observations,
            expected_fingerprints,
        )?;
        let bytes = retained.journal.encode()?;
        let journal = FrozenRetainedJournal::from_descriptor(
            &archive::hash(&bytes),
            &retained.store_identity,
            retained.snapshot_sequence,
            &bytes,
        )?;
        let rows = assess(
            dataset,
            profile,
            &journal,
            &closure,
            retained.captured_at,
            roster_context,
            replay.independent(),
        )?;
        let documentary =
            replay_documentary(dataset, profile, retained.captured_at, &replay, directory)?;
        Ok(Self {
            input_generation: dataset.lineage.input_generation.clone(),
            profile_id: profile.id().map_err(StoreError::from)?,
            participation_rule: profile.input().policies.participation.clone(),
            rows,
            documentary: documentary.rows,
            unsupported_documentary: documentary.unsupported,
            replay,
        })
    }
}

fn seal_live(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    observations: Vec<PhysicalObservationDescriptor>,
    recorder: census_crawl::milesplit::RecordingRetainedJournal<'_, '_>,
    independent: census_crawl::participation_evidence::IndependentParticipationBundleV1,
    rows: BTreeMap<PerformanceId, PinnedParticipationRow>,
    directory: &Path,
) -> ReportResult<PinnedParticipation> {
    let replay = ParticipationReplayV1 {
        version: 1,
        store_identity: dataset.lineage.store_identity.clone(),
        snapshot_sequence: snapshot.sequence(),
        captured_at,
        input_generation: dataset.lineage.input_generation.clone(),
        input_digest: dataset.lineage.input_digest.clone(),
        source_digest: dataset.lineage.source_digest.clone(),
        profile_id: profile.id().map_err(StoreError::from)?,
        profile: profile.clone(),
        observations,
        journal: recorder.finish()?,
    };
    let (documentary, attachment) =
        attached::capture(dataset, snapshot, profile, captured_at, directory)?;
    let replay = ParticipationReplay::V4(ParticipationReplayV4 {
        version: 4,
        retained: ParticipationReplayV2 {
            version: 2,
            retained: replay,
            independent,
        },
        attachment,
    });
    Ok(PinnedParticipation {
        input_generation: dataset.lineage.input_generation.clone(),
        profile_id: profile.id().map_err(StoreError::from)?,
        participation_rule: profile.input().policies.participation.clone(),
        rows,
        documentary: documentary.rows,
        unsupported_documentary: documentary.unsupported,
        replay,
    })
}

fn replay_documentary(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    captured_at: DateTime<Utc>,
    replay: &ParticipationReplay,
    directory: &Path,
) -> ReportResult<attached::Collected> {
    match replay {
        ParticipationReplay::V3(value) => {
            attached::replay(dataset, profile, captured_at, &value.attachment, directory)
        }
        ParticipationReplay::V4(value) => {
            attached::replay_current(dataset, profile, captured_at, &value.attachment, directory)
        }
        ParticipationReplay::V1(_) | ParticipationReplay::V2(_) => Ok(attached::Collected {
            rows: BTreeMap::new(),
            unsupported: Vec::new(),
        }),
    }
}

fn assess(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    view: &impl RetainedJournalView,
    closure: &closure::Closure,
    captured_at: DateTime<Utc>,
    roster_context: VerifiedRosterContext<'_>,
    independent: Option<&census_crawl::participation_evidence::IndependentParticipationBundleV1>,
) -> ReportResult<BTreeMap<PerformanceId, PinnedParticipationRow>> {
    let profile_id = profile.id().map_err(StoreError::from)?;
    let frozen = DeclaredFrozenView {
        lineage: object(&dataset.lineage.store_identity)?,
        revision: NonZeroU64::new(view.sequence())
            .ok_or_else(|| invalid("empty participation snapshot revision"))?,
        captured_at,
        closure_object: object(&dataset.lineage.input_digest)?,
        family_inventory: object(&dataset.lineage.source_digest)?,
    };
    let query = CandidateQuery {
        profile,
        binding: CandidateBinding {
            profile: &profile_id,
            participation_rule: &profile.input().policies.participation,
            frozen,
        },
    };
    let mut resolver = resolve::Resolver {
        dataset,
        identities: crate::export::public_identity::population::IdentityPopulation::of(dataset)?,
        sources: RetainedResultSession::new(
            view,
            CaptureReadBudget::new(32 * 1024 * 1024)
                .map_err(|_| invalid("participation read budget invalid"))?,
        ),
        contexts: contexts::ResultContexts::default(),
        closure,
        roster_context,
        budget: budget::Captures::default(),
        independent,
    };
    Ok(resolver.grouped_rows(query)?)
}

fn object(value: &str) -> StoreResult<ObjectRef<'_>> {
    ObjectRef::parse(value)
        .map_err(|_| invalid("participation snapshot object reference is invalid"))
}
fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}
fn charge(current: &mut u64, added: usize, maximum: u64, what: &'static str) -> StoreResult<()> {
    let needed = current
        .checked_add(u64::try_from(added).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > maximum {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available: maximum,
        });
    }
    *current = needed;
    Ok(())
}
#[cfg(test)]
mod tests;
