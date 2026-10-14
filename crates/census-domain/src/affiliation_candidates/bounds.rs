use std::num::{NonZeroU64, NonZeroUsize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectRef<'a>(&'a str);

impl<'a> ObjectRef<'a> {
    pub fn parse(value: &'a str) -> Result<Self, AffiliationCandidateError> {
        if value.is_empty()
            || value.len() > 512
            || value.chars().any(char::is_whitespace)
            || value.chars().any(char::is_control)
        {
            return Err(AffiliationCandidateError::InvalidObjectReference);
        }
        Ok(Self(value))
    }

    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownBounds<'a> {
    lineage: ObjectRef<'a>,
    first: NonZeroU64,
    through: NonZeroU64,
}

impl<'a> KnownBounds<'a> {
    pub fn new(
        lineage: ObjectRef<'a>,
        first: NonZeroU64,
        through: NonZeroU64,
    ) -> Result<Self, AffiliationCandidateError> {
        if first > through {
            return Err(AffiliationCandidateError::ReversedKnownBounds);
        }
        Ok(Self {
            lineage,
            first,
            through,
        })
    }

    pub fn visible(
        self,
        lineage: ObjectRef<'_>,
        revision: NonZeroU64,
    ) -> Result<bool, AffiliationCandidateError> {
        if self.lineage != lineage {
            return Err(AffiliationCandidateError::MixedKnownLineage);
        }
        Ok(self.first <= revision && revision <= self.through)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resource {
    Assertions,
    Changes,
    AuditFacts,
    BodyBytes,
    MetadataBytes,
    ComponentBytes,
    WorkUnits,
    Targets,
    PublishedOwners,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionGrant {
    pub(crate) assertions: usize,
    pub(crate) changes: usize,
    pub(crate) facts: usize,
    pub(crate) bytes: usize,
    pub(crate) work: usize,
}

impl SelectionGrant {
    pub fn new(
        assertions: NonZeroUsize,
        changes: NonZeroUsize,
        facts: NonZeroUsize,
        bytes: NonZeroUsize,
        work: NonZeroUsize,
    ) -> Result<Self, AffiliationCandidateError> {
        check_limit(Resource::Assertions, assertions.get(), 256)?;
        check_limit(Resource::Changes, changes.get(), 512)?;
        check_limit(Resource::AuditFacts, facts.get(), 768)?;
        check_limit(Resource::ComponentBytes, bytes.get(), 8_388_608)?;
        check_limit(Resource::WorkUnits, work.get(), 2_000_000)?;
        Ok(Self {
            assertions: assertions.get(),
            changes: changes.get(),
            facts: facts.get(),
            bytes: bytes.get(),
            work: work.get(),
        })
    }
}

pub(crate) fn check_limit(
    resource: Resource,
    needed: usize,
    maximum: usize,
) -> Result<(), AffiliationCandidateError> {
    if needed > maximum {
        return Err(AffiliationCandidateError::BudgetExceeded {
            resource,
            needed,
            maximum,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AffiliationCandidateError {
    #[error("invalid bounded object reference")]
    InvalidObjectReference,
    #[error("known-at bounds are reversed")]
    ReversedKnownBounds,
    #[error("known-at lineages differ")]
    MixedKnownLineage,
    #[error("claimed capture body or manifest is missing")]
    MissingCaptureMaterial,
    #[error("claimed interpretation record is missing")]
    MissingInterpretationMaterial,
    #[error("capture owner disagrees with source subject")]
    ForeignCaptureOwner,
    #[error("capture URL or source label is invalid")]
    InvalidCaptureSource,
    #[error("capture acquisition instant disagrees with legacy date")]
    AcquisitionDateMismatch,
    #[error("capture length differs from supplied body")]
    LengthMismatch,
    #[error("capture digest differs from supplied body")]
    HashMismatch,
    #[error("source span is empty, missing or too large")]
    InvalidLocator,
    #[error("requested, observed or published owner disagrees")]
    ForeignOwner,
    #[error("retained published governing owner declarations contradict each other")]
    ConflictingAuthoritativeOwners,
    #[error("no published governing owner is retained")]
    MissingPublishedOwner,
    #[error("owner institution/program and claimed relationship disagree")]
    RelationshipOwnerMismatch,
    #[error("co-op campus binding names a different or missing source person")]
    CoopPersonMismatch,
    #[error("identity or institution ID does not validate under its existing type")]
    InvalidExistingId,
    #[error("unknown sport is not an admitted scope")]
    UnknownSport,
    #[error("calendar year/sport does not match the claimed season")]
    CalendarScopeMismatch,
    #[error("capture is later than the declared frozen instant")]
    AcquisitionAfterFrozenInstant,
    #[error("capture is later than the declared acquisition cutoff")]
    AcquisitionAfterDeclaredCutoff,
    #[error("declared acquisition-through day exceeds effective as_of under the current compiler envelope")]
    CurrentCompilerEnvelopeViolated,
    #[error("one observation/change ID names different payloads")]
    DuplicateIdDifferentPayload,
    #[error("change target is absent or belongs to a different subject")]
    DanglingOrForeignChangeTarget,
    #[error("correction, activation and reviewed-resolution policy is unresolved")]
    UnsupportedChange,
    #[error("withdrawal and departure apply to the same assertion")]
    ConflictingChanges,
    #[error("known bounds do not agree with dependency visibility")]
    DependencyNotVisible,
    #[error("{resource:?} needs {needed}, maximum {maximum}")]
    BudgetExceeded {
        resource: Resource,
        needed: usize,
        maximum: usize,
    },
    #[error("bounded allocation failed")]
    AllocationFailed,
    #[error("resource counter overflow")]
    CounterOverflow,
}
