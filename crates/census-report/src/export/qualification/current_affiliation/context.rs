mod membership;

use super::{charge, invalid, CurrentAffiliationResolution, PinnedCurrentAffiliations};
use crate::export::ExportDataset;
use census_crawl::milesplit::CurrentRosterAssertion;
use census_domain::model::{serialized_digest, AthleteId, Evidence, ProfileDate, QueryProfileV1};
use census_store::StoreResult;
use membership::DatedMembership;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservationDayConvention {
    UtcCaptureDay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DatedRosterMembership {
    pub(crate) observed_on: ProfileDate,
    pub(crate) observed_at: String,
    pub(crate) day_convention: ObservationDayConvention,
    pub(crate) assertion: CurrentRosterAssertion,
    pub(crate) capture_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Membership {
    Physical,
    Dated(DatedMembership),
}

#[derive(Default)]
pub(super) struct RosterEvidence {
    rows: BTreeMap<AthleteId, BTreeMap<String, Membership>>,
    count: u64,
    bytes: u64,
}

impl RosterEvidence {
    pub(super) fn retain(&mut self, athlete: &AthleteId, evidence: &Evidence) -> StoreResult<()> {
        let digest = serialized_digest(evidence)?;
        let rows = self.rows.get(athlete);
        if rows.is_some_and(|rows| rows.contains_key(&digest)) {
            return Ok(());
        }
        if rows.is_some_and(|rows| rows.len() >= 256) {
            return Err(invalid(
                "verified roster context exceeds 256 proofs per physical subject",
            ));
        }
        let mut count = self.count;
        let mut retained_bytes = self.bytes;
        charge(
            &mut count,
            1,
            1_000_000,
            "verified roster context proof count",
        )?;
        let bytes = athlete
            .as_str()
            .len()
            .checked_add(digest.len())
            .ok_or(census_store::StoreError::CounterOverflow)?;
        charge(
            &mut retained_bytes,
            bytes,
            64 * 1024 * 1024,
            "verified roster context encoded bytes",
        )?;
        self.rows
            .entry(athlete.clone())
            .or_default()
            .insert(digest, Membership::Physical);
        self.count = count;
        self.bytes = retained_bytes;
        Ok(())
    }

    pub(super) fn retain_dated(
        &mut self,
        athlete: &AthleteId,
        evidence: &Evidence,
        proof: DatedRosterMembership,
    ) -> StoreResult<()> {
        let digest = serialized_digest(evidence)?;
        let row = self
            .rows
            .get_mut(athlete)
            .and_then(|rows| rows.get_mut(&digest))
            .ok_or_else(|| invalid("dated membership lacks checked physical context"))?;
        if let Membership::Dated(previous) = row {
            if previous.get()? != &proof {
                return Err(invalid(
                    "dated roster membership changed for identical evidence",
                ));
            }
            return Ok(());
        }
        let bytes = [
            std::mem::size_of::<DatedRosterMembership>(),
            proof.observed_on.as_str().len(),
            proof.observed_at.len(),
            proof.capture_sha256.len(),
            proof.assertion.owner().len(),
            proof.assertion.member().len(),
        ]
        .into_iter()
        .try_fold(0usize, |sum, value| sum.checked_add(value))
        .ok_or(census_store::StoreError::CounterOverflow)?;
        let mut retained_bytes = self.bytes;
        charge(
            &mut retained_bytes,
            bytes,
            64 * 1024 * 1024,
            "verified roster context encoded bytes",
        )?;
        *row = Membership::Dated(DatedMembership::new(proof)?);
        self.bytes = retained_bytes;
        Ok(())
    }

    pub(super) fn dated_membership(
        &self,
        athlete: &AthleteId,
        evidence: &Evidence,
    ) -> StoreResult<Option<&DatedRosterMembership>> {
        let digest = serialized_digest(evidence)?;
        Ok(
            match self.rows.get(athlete).and_then(|rows| rows.get(&digest)) {
                Some(Membership::Dated(proof)) => Some(proof.get()?),
                Some(Membership::Physical) | None => None,
            },
        )
    }

    fn contains(&self, athlete: &AthleteId, evidence: &Evidence) -> StoreResult<bool> {
        let digest = serialized_digest(evidence)?;
        Ok(self
            .rows
            .get(athlete)
            .is_some_and(|rows| rows.contains_key(&digest)))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct VerifiedRosterContext<'a> {
    affiliation: &'a PinnedCurrentAffiliations,
}

impl PinnedCurrentAffiliations {
    pub(crate) fn roster_context(&self) -> VerifiedRosterContext<'_> {
        VerifiedRosterContext { affiliation: self }
    }
}

impl VerifiedRosterContext<'_> {
    pub(crate) fn require_binding(
        &self,
        dataset: &ExportDataset,
        profile: &QueryProfileV1,
    ) -> StoreResult<()> {
        if self.affiliation.input_generation != dataset.lineage.input_generation
            || self.affiliation.profile_id != profile.id()?
            || self.affiliation.as_of != profile.input().as_of
        {
            return Err(invalid(
                "roster context differs from the pinned participation input",
            ));
        }
        Ok(())
    }

    pub(in crate::export::qualification) fn contains(
        &self,
        athlete: &AthleteId,
        evidence: &Evidence,
    ) -> StoreResult<bool> {
        if self.dated_membership(athlete, evidence)?.is_some() {
            return Ok(true);
        }
        self.affiliation.audit.context.contains(athlete, evidence)
    }

    pub(in crate::export::qualification) fn dated_membership(
        &self,
        athlete: &AthleteId,
        evidence: &Evidence,
    ) -> StoreResult<Option<&DatedRosterMembership>> {
        self.affiliation
            .audit
            .context
            .dated_membership(athlete, evidence)
    }

    pub(in crate::export::qualification) fn conflicted(&self, candidate: &str) -> bool {
        self.affiliation.rows.get(candidate) == Some(&CurrentAffiliationResolution::Conflicted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::SourceRef;
    use census_store::StoreError;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn evidence(note: &str) -> Evidence {
        Evidence::derived(SourceRef::id("context-test"), "2026-10-01", note)
    }

    #[test]
    fn aggregate_boundary_accepts_duplicate_but_refusal_preserves_roster_state() -> TestResult {
        let athlete = AthleteId::mint("ath_subject", &["context"]);
        let proof = evidence("proof");
        let bytes = u64::try_from(athlete.as_str().len() + serialized_digest(&proof)?.len())?;
        let mut context = RosterEvidence {
            count: 999_999,
            bytes: 64 * 1024 * 1024 - bytes,
            ..Default::default()
        };
        context.retain(&athlete, &proof)?;
        let before = context.rows.clone();
        context.retain(&athlete, &proof)?;
        let other = AthleteId::mint("ath_subject", &["foreign"]);
        assert!(matches!(
            context.retain(&other, &proof),
            Err(StoreError::BudgetExceeded {
                what: "verified roster context proof count",
                needed: 1_000_001,
                available: 1_000_000
            })
        ));
        assert_eq!(context.rows, before);
        assert_eq!(
            (context.count, context.bytes),
            (1_000_000, 64 * 1024 * 1024)
        );
        Ok(())
    }

    #[test]
    fn byte_refusal_does_not_insert_roster_subject_or_charge_count() -> TestResult {
        let athlete = AthleteId::mint("ath_subject", &["context"]);
        let proof = evidence("proof");
        let added = u64::try_from(athlete.as_str().len() + serialized_digest(&proof)?.len())?;
        let mut context = RosterEvidence {
            count: 7,
            bytes: 64 * 1024 * 1024,
            ..Default::default()
        };
        assert!(
            matches!(context.retain(&athlete, &proof), Err(StoreError::BudgetExceeded {
            what: "verified roster context encoded bytes", needed, available: 67_108_864
        }) if needed == 67_108_864 + added)
        );
        assert!(context.rows.is_empty());
        assert_eq!((context.count, context.bytes), (7, 67_108_864));
        Ok(())
    }

    #[test]
    fn physical_subject_limit_preserves_roster_state_and_duplicate_identity() -> TestResult {
        let athlete = AthleteId::mint("ath_subject", &["context"]);
        let mut context = RosterEvidence::default();
        for ordinal in 0..256 {
            context.retain(&athlete, &evidence(&ordinal.to_string()))?;
        }
        let before = (context.rows.clone(), context.count, context.bytes);
        context.retain(&athlete, &evidence("0"))?;
        assert!(matches!(context.retain(&athlete, &evidence("extra")),
            Err(StoreError::Refused { detail })
            if detail == "verified roster context exceeds 256 proofs per physical subject"));
        assert_eq!((context.rows, context.count, context.bytes), before);
        Ok(())
    }
}
