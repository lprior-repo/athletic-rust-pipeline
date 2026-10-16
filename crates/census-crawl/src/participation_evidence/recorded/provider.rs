use super::super::provider_target::{
    CheckedMileSplitProfileSubjectV1, CheckedProviderTargetIntakeV1, CompetitionCategoryEvidenceV1,
    ProviderTargetProofV1,
};
use super::super::{bound, codec, invalid, MAX_PACKET};
use crate::milesplit::ProviderSchools;
use crate::recording::RowBatch;
use crate::CrawlResult;
use census_domain::model::{
    AthleteId, CanonicalAthlete, Evidence, Gender, PublishedGraduation, SourceAthleteObservation,
    SourceObservation, SourceRef,
};
use census_store::RetainedCaptureRootSchemaV1;
use census_store::{StoreError, Table};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTargetWithheldV1 {
    CompetitionCategoryUnavailable,
    CompetitionCategoryConflicted,
    SchoolOwnershipUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTargetIntakeDispositionV1 {
    ProjectedUnaccepted { candidate: AthleteId },
    RetainedOnly(ProviderTargetWithheldV1),
}

pub(super) struct Projection {
    pub(super) disposition: ProviderTargetIntakeDispositionV1,
    athlete: Option<CanonicalAthlete>,
    observations: Vec<SourceObservation>,
}

impl Projection {
    pub(super) fn append(&self, batch: &mut RowBatch<'_>) -> CrawlResult<()> {
        codec::encode(&self.observations, MAX_PACKET)?;
        batch.append_many(Table::SourceObservations, &self.observations)?;
        if let Some(athlete) = &self.athlete {
            codec::encode(athlete, MAX_PACKET)?;
            batch.append_many(Table::Athletes, std::slice::from_ref(athlete))?;
        }
        Ok(())
    }
}

pub(super) fn project(
    checked: &CheckedProviderTargetIntakeV1,
    schools: &ProviderSchools,
) -> CrawlResult<Projection> {
    let observations = observations(checked)?;
    let category = match checked.categories() {
        CompetitionCategoryEvidenceV1::Agreed { category, .. } => *category,
        CompetitionCategoryEvidenceV1::Unavailable { .. } => {
            return Ok(retained(
                ProviderTargetWithheldV1::CompetitionCategoryUnavailable,
                observations,
            ))
        }
        CompetitionCategoryEvidenceV1::Conflicted { .. } => {
            return Ok(retained(
                ProviderTargetWithheldV1::CompetitionCategoryConflicted,
                observations,
            ))
        }
    };
    let school_id =
        checked.profile().school().id.parse::<u64>().map_err(|_| {
            invalid("checked provider school ID does not fit its canonical resolver")
        })?;
    let Some(school) = schools.resolve(school_id) else {
        return Ok(retained(
            ProviderTargetWithheldV1::SchoolOwnershipUnresolved,
            observations,
        ));
    };
    let athlete = athlete(checked, school, category)?;
    Ok(Projection {
        disposition: ProviderTargetIntakeDispositionV1::ProjectedUnaccepted {
            candidate: athlete.id.clone(),
        },
        athlete: Some(athlete),
        observations,
    })
}

fn retained(reason: ProviderTargetWithheldV1, observations: Vec<SourceObservation>) -> Projection {
    Projection {
        disposition: ProviderTargetIntakeDispositionV1::RetainedOnly(reason),
        athlete: None,
        observations,
    }
}

fn observations(checked: &CheckedProviderTargetIntakeV1) -> CrawlResult<Vec<SourceObservation>> {
    let rows = checked.categories().rows();
    bound(rows.len(), 64, "provider target linked observations")?;
    let count = rows
        .len()
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let mut observations = Vec::new();
    observations
        .try_reserve_exact(count)
        .map_err(|_| invalid("provider target observation allocation refused"))?;
    let profile = checked.profile();
    let proof = profile
        .proofs()
        .first()
        .ok_or_else(|| invalid("checked profile has no subject proof"))?;
    observations.push(observation(
        checked,
        proof,
        "profile",
        profile.competition_category(),
    ));
    for row in rows {
        observations.push(observation(
            checked,
            row.proof(),
            row.source_scope(),
            match row.category() {
                Some(category) => category,
                None => Gender::Unknown,
            },
        ));
    }
    Ok(observations)
}

fn observation(
    checked: &CheckedProviderTargetIntakeV1,
    proof: &ProviderTargetProofV1,
    scope: &str,
    category: Gender,
) -> SourceObservation {
    let profile = checked.profile();
    SourceObservation::Athlete(
        SourceAthleteObservation::new(
            profile.person().namespace.clone(),
            profile.person().id.clone(),
            format!(
                "{}:{}:{scope}:{}",
                RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
                checked.reference().as_str(),
                proof.locator()
            ),
            profile.name(),
            proof.acquired_at().to_rfc3339(),
        )
        .with_gender(category)
        .with_profile_url(profile.person().url.clone())
        .with_school(Some(profile.school_name().into())),
    )
}

fn athlete(
    checked: &CheckedProviderTargetIntakeV1,
    school: &census_domain::model::SchoolId,
    category: Gender,
) -> CrawlResult<CanonicalAthlete> {
    let profile = checked.profile();
    let mut athlete = CanonicalAthlete::new(
        school,
        profile.name(),
        profile.cohort(),
        category,
        profile.person().clone(),
    )?;
    athlete
        .public_profile_urls
        .try_reserve_exact(1)
        .map_err(|_| invalid("provider target profile allocation refused"))?;
    athlete
        .public_profile_urls
        .extend(profile.person().url.clone());
    athlete
        .published_graduations
        .try_reserve_exact(1)
        .map_err(|_| invalid("provider target graduation allocation refused"))?;
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: profile.cohort(),
        source: profile_source(profile),
    });
    athlete.evidence = evidence(checked)?;
    Ok(athlete)
}

fn evidence(checked: &CheckedProviderTargetIntakeV1) -> CrawlResult<Vec<Evidence>> {
    let profile = checked.profile();
    let count = checked
        .categories()
        .rows()
        .len()
        .checked_mul(3)
        .and_then(|rows| rows.checked_add(profile.proofs().len()))
        .and_then(|rows| rows.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    bound(count, 197, "provider target evidence rows")?;
    let mut evidence = Vec::new();
    evidence
        .try_reserve_exact(count)
        .map_err(|_| invalid("provider target evidence allocation refused"))?;
    for proof in profile.proofs() {
        evidence.push(proof_evidence(
            checked,
            proof,
            "provider_profile",
            profile_source(profile),
        )?);
    }
    evidence.push(proof_evidence(
        checked,
        profile.person_gender().proof(),
        profile.person_gender().source_scope(),
        profile_source(profile),
    )?);
    for row in checked.categories().rows() {
        for proof in [row.proof(), row.context_proof(), row.school_proof()] {
            evidence.push(proof_evidence(
                checked,
                proof,
                row.source_scope(),
                SourceRef::id("milesplit_mi"),
            )?);
        }
    }
    Ok(evidence)
}

fn profile_source(profile: &CheckedMileSplitProfileSubjectV1) -> SourceRef {
    SourceRef::new("milesplit_mi", profile.person().url.clone())
}

#[derive(Serialize)]
struct ProofNote<'a> {
    version: u32,
    phase: &'static str,
    packet: &'a str,
    source_scope: &'a str,
    capture: &'a str,
    interpretation: &'a str,
    locator: &'a str,
}

fn proof_evidence(
    checked: &CheckedProviderTargetIntakeV1,
    proof: &ProviderTargetProofV1,
    scope: &str,
    source: SourceRef,
) -> CrawlResult<Evidence> {
    let bytes = codec::encode(
        &ProofNote {
            version: 1,
            phase: RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            packet: checked.reference().as_str(),
            source_scope: scope,
            capture: proof.capture(),
            interpretation: proof.interpretation(),
            locator: proof.locator(),
        },
        4096,
    )?;
    let note =
        String::from_utf8(bytes).map_err(|_| invalid("provider target proof JSON is not UTF-8"))?;
    let mut evidence = Evidence::parsed(source, proof.acquired_at().to_rfc3339());
    evidence.note = Some(note);
    Ok(evidence)
}
