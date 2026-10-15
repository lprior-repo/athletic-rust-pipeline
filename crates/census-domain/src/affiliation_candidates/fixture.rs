type TestResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;
use crate::affiliation_candidates::*;
use crate::model::{
    AthleteId, ContractRefV1, ProfileDate, ProfileDateRangeV1, SchoolId, SourceCapture,
    SourceNamespace, SourceRef, SourceSubject, Sport, TeamId,
};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::num::{NonZeroU64, NonZeroUsize};

pub(super) const BODY: &[u8] = b"SYNTHETIC ONLY: owner A; person 700001; enrolled A 2026-09-01 through 2027-06-15; other example B; withdrawal A; last day A 2026-09-30";

pub(super) fn key(value: &str) -> TestResult<ObjectRef<'_>> {
    Ok(ObjectRef::parse(value)?)
}
pub(super) fn day(value: &str) -> TestResult<ProfileDate> {
    Ok(ProfileDate::parse(value)?)
}
pub(super) fn instant(value: &str) -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
pub(super) fn revision(value: u64) -> TestResult<NonZeroU64> {
    Ok(NonZeroU64::new(value).ok_or("zero synthetic revision")?)
}
pub(super) fn size(value: usize) -> TestResult<NonZeroUsize> {
    Ok(NonZeroUsize::new(value).ok_or("zero synthetic grant")?)
}
pub(super) fn known(first: u64, through: u64) -> TestResult<KnownBounds<'static>> {
    Ok(KnownBounds::new(
        key("synthetic-lineage")?,
        revision(first)?,
        revision(through)?,
    )?)
}
pub(super) fn grant() -> TestResult<SelectionGrant> {
    Ok(SelectionGrant::new(
        size(256)?,
        size(512)?,
        size(768)?,
        size(8_388_608)?,
        size(2_000_000)?,
    )?)
}

pub(super) struct Fixture {
    pub athlete: AthleteId,
    pub other_athlete: AthleteId,
    pub school: SchoolId,
    pub other_school: SchoolId,
    pub program: TeamId,
    pub owner: SourceSubject,
    pub other_owner: SourceSubject,
    pub person: SourceSubject,
    pub source: SourceRef,
    pub metadata: SourceCapture,
    pub interval: ProfileDateRangeV1,
    pub as_of: ProfileDate,
    pub rule: ContractRefV1,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, "800001".to_owned())?;
        let metadata = SourceCapture::new(
            &owner,
            format!("{:x}", Sha256::digest(BODY)),
            u64::try_from(BODY.len())?,
            "2026-08-01".to_owned(),
            Some("https://roster.example.test/requested".to_owned()),
        )?;
        Ok(Self {
            athlete: AthleteId::mint("ath_subject", &["synthetic-700001"]),
            other_athlete: AthleteId::mint("ath_subject", &["synthetic-700002"]),
            school: SchoolId::mint("sch", &["synthetic-A"]),
            other_school: SchoolId::mint("sch", &["synthetic-B"]),
            program: TeamId::mint("team", &["synthetic-coop"]),
            owner,
            other_owner: SourceSubject::new(SourceNamespace::MilesplitSchool, "800002".to_owned())?,
            person: SourceSubject::new(SourceNamespace::MilesplitAthlete, "700001".to_owned())?,
            source: SourceRef::id("synthetic-roster"),
            metadata,
            interval: ProfileDateRangeV1::new(day("2026-09-01")?, day("2027-06-15")?)?,
            as_of: day("2026-10-10")?,
            rule: ContractRefV1::new(1, 1, &"0".repeat(64))?,
        })
    }

    pub fn evidence(&self) -> TestResult<LocallyCheckedEvidence<'_>> {
        Ok(LocallyCheckedEvidence::check(RawEvidence {
            occurrence: key("synthetic-occurrence-A")?,
            manifest_key: key("synthetic-manifest-A")?,
            manifest_bytes: b"synthetic manifest A",
            metadata: &self.metadata,
            owner: &self.owner,
            source: &self.source,
            requested_url: "https://roster.example.test/requested",
            final_url: FinalUrl::Unknown,
            acquired_at: instant("2026-08-01T12:00:00Z")?,
            body: BODY,
            interpretation_key: key("synthetic-interpretation-A")?,
            interpretation_bytes: b"synthetic extraction record A",
            parser_artifact: key("synthetic-parser")?,
            extraction_schema: key("synthetic-schema")?,
            extraction_config: key("synthetic-config")?,
            reference_dependencies: key("synthetic-references")?,
        })?)
    }

    pub fn assertion<'a>(
        &'a self,
        evidence: &'a LocallyCheckedEvidence<'a>,
        published: &'a [&'a SourceSubject],
        id: ObjectRef<'a>,
    ) -> TestResult<RawAssertion<'a>> {
        let span = SourceSpan::new(0, BODY.len())?;
        Ok(RawAssertion {
            id,
            evidence,
            roster_qualification_claim: key("synthetic-qualification-UNVERIFIED")?,
            source_activation_claim: key("synthetic-activation-UNVERIFIED")?,
            row_locator: key("synthetic-row-A")?,
            person: self.person_candidate()?,
            owner: RawOwnership {
                requested: &self.owner,
                observed: ObservedOwner::Unknown,
                published,
                governing: &self.owner,
                target: OwnerTarget::School(&self.school),
                binding_decision: key("synthetic-owner-binding-UNVERIFIED")?,
                known: known(1, 3)?,
            },
            relationship: RelationshipCandidate::EnrolledAtSchool(&self.school),
            temporal: TemporalScope::Closed(&self.interval),
            sport: SportScope::Sport(Sport::CrossCountry),
            known: known(1, 3)?,
            membership_statement: span,
            person_statement: span,
            owner_statement: span,
            temporal_statement: span,
        })
    }

    pub fn person_candidate(&self) -> TestResult<PersonCandidate<'_>> {
        Ok(PersonCandidate::Mapped {
            person: &self.person,
            athlete: &self.athlete,
            decision: key("synthetic-identity-binding-UNVERIFIED")?,
            known: known(1, 3)?,
        })
    }

    pub fn query(&self) -> TestResult<CandidateQuery<'_>> {
        Ok(CandidateQuery {
            athlete: &self.athlete,
            as_of: &self.as_of,
            sport: Sport::CrossCountry,
            known: DeclaredFrozenView {
                lineage: key("synthetic-lineage")?,
                revision: revision(1)?,
                captured_at: instant("2026-10-10T12:00:00Z")?,
                closure_object: key("synthetic-closure-UNVERIFIED")?,
                family_inventory: key("synthetic-inventory-UNVERIFIED")?,
            },
            acquisition_cutoff: instant("2026-08-01T12:00:00Z")?,
            rule_claim: &self.rule,
            grant: grant()?,
        })
    }

    pub fn change<'a>(
        &'a self,
        evidence: &'a LocallyCheckedEvidence<'a>,
        target: ObjectRef<'a>,
        kind: ChangeKind<'a>,
    ) -> TestResult<LocallyCheckedChange<'a>> {
        Ok(LocallyCheckedChange::check(RawChange {
            id: key("synthetic-change-A")?,
            target,
            person: self.person_candidate()?,
            kind,
            evidence,
            locator: key("synthetic-change-row")?,
            reason_statement: SourceSpan::new(0, BODY.len())?,
            known: known(1, 3)?,
        })?)
    }
}
