use super::{
    AffiliationCandidateError, KnownBounds, LocallyCheckedEvidence, ObjectRef, PersonCandidate,
    SourceSpan,
};
use crate::model::{ProfileDate, ProfileDateRangeV1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind<'a> {
    WithdrawsClaim {
        scope: &'a ProfileDateRangeV1,
    },
    EndsRelationship {
        last_day: &'a ProfileDate,
    },
    Corrects {
        replacement: ObjectRef<'a>,
        scope: &'a ProfileDateRangeV1,
    },
    SourceActivation {
        predecessor: ObjectRef<'a>,
    },
    ReviewedResolution {
        decision: ObjectRef<'a>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawChange<'a> {
    pub id: ObjectRef<'a>,
    pub target: ObjectRef<'a>,
    pub person: PersonCandidate<'a>,
    pub kind: ChangeKind<'a>,
    pub evidence: &'a LocallyCheckedEvidence<'a>,
    pub locator: ObjectRef<'a>,
    pub reason_statement: SourceSpan,
    pub known: KnownBounds<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocallyCheckedChange<'a> {
    raw: RawChange<'a>,
}

impl<'a> LocallyCheckedChange<'a> {
    pub fn check(raw: RawChange<'a>) -> Result<Self, AffiliationCandidateError> {
        raw.reason_statement.read(raw.evidence)?;
        if let PersonCandidate::Mapped { athlete, .. } = raw.person {
            if athlete.as_str().is_empty() || athlete.as_str().len() > 512 {
                return Err(AffiliationCandidateError::InvalidExistingId);
            }
        }
        Ok(Self { raw })
    }

    pub const fn raw(&self) -> &RawChange<'a> {
        &self.raw
    }
}
