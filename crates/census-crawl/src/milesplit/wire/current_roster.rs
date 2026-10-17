use super::SourceRowLocator;
use census_domain::affiliation_candidates::SourceSpan;
use census_domain::UsJurisdiction;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentRosterAssertion {
    pub(in crate::milesplit) owner: Box<str>,
    pub(in crate::milesplit) member: Box<str>,
    pub(in crate::milesplit) row: SourceRowLocator,
    pub(in crate::milesplit) section: std::ops::Range<usize>,
    pub(in crate::milesplit) container: std::ops::Range<usize>,
    pub(in crate::milesplit) heading: std::ops::Range<usize>,
    pub(in crate::milesplit) heading_sha256: [u8; 32],
    pub(in crate::milesplit) sports: [bool; 3],
    pub(in crate::milesplit) statements: Result<RosterRowStatementsV1, RosterWitnessUnavailableV1>,
}

impl CurrentRosterAssertion {
    pub const INTERPRETATION_REVISION: &'static str = "milesplit-scoped-current-roster/v1";

    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn member(&self) -> &str {
        &self.member
    }
    pub fn row(&self) -> SourceRowLocator {
        self.row
    }
    pub fn section(&self) -> std::ops::Range<usize> {
        self.section.clone()
    }
    pub fn container(&self) -> std::ops::Range<usize> {
        self.container.clone()
    }
    pub fn heading(&self) -> std::ops::Range<usize> {
        self.heading.clone()
    }
    pub fn heading_sha256(&self) -> &[u8; 32] {
        &self.heading_sha256
    }

    pub fn source_statements_v1(
        &self,
    ) -> Result<&RosterRowStatementsV1, RosterWitnessUnavailableV1> {
        self.statements.as_ref().map_err(|reason| *reason)
    }

    pub fn supports(&self, sport: census_domain::model::Sport) -> bool {
        use census_domain::model::Sport;
        let [indoor, outdoor, xc] = self.sports;
        match sport {
            Sport::IndoorTrack => indoor,
            Sport::OutdoorTrack => outdoor,
            Sport::CrossCountry => xc,
            Sport::Unknown => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterTemporalUnavailable {
    MissingStatement,
    AmbiguousStatement,
    MalformedMarkup,
    ResourceLimit,
    InactiveMember,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RosterTemporalEvidence {
    Unavailable(RosterTemporalUnavailable),
    Current(CurrentRosterAssertion),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RosterWitnessUnavailableV1 {
    #[error("published roster owner declaration is absent")]
    MissingPublishedOwner,
    #[error("roster statement has no exact active source span")]
    MissingSourceSpan,
    #[error("roster statement source span exceeds 4096 bytes")]
    SourceSpanLimit,
    #[error("published roster owner inventory exceeds 32 declarations")]
    OwnerInventoryLimit,
    #[error("roster source witness allocation refused")]
    ResourceLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterRowStatementsV1 {
    person: SourceSpan,
    membership: SourceSpan,
    temporal: SourceSpan,
}

impl RosterRowStatementsV1 {
    pub(in crate::milesplit) fn checked(
        html: &str,
        person: Range<usize>,
        membership: Range<usize>,
        temporal: Range<usize>,
    ) -> Result<Self, RosterWitnessUnavailableV1> {
        if person.start < membership.start || person.end > membership.end {
            return Err(RosterWitnessUnavailableV1::MissingSourceSpan);
        }
        Ok(Self {
            person: checked_span(html, person)?,
            membership: checked_span(html, membership)?,
            temporal: checked_span(html, temporal)?,
        })
    }

    pub fn person(&self) -> SourceSpan {
        self.person
    }
    pub fn membership(&self) -> SourceSpan {
        self.membership
    }
    pub fn temporal(&self) -> SourceSpan {
        self.temporal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedRosterOwnerKindV1 {
    Canonical,
    OpenGraph,
    Twitter,
    JsonLdUrl,
    JsonLdId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedRosterOwnerTerminationV1 {
    Tag,
    ScriptEnd,
    ScriptEof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRosterOwnerV1 {
    provider_id: u64,
    jurisdiction: Option<UsJurisdiction>,
    kind: PublishedRosterOwnerKindV1,
    span: SourceSpan,
    termination: PublishedRosterOwnerTerminationV1,
}

impl PublishedRosterOwnerV1 {
    pub(in crate::milesplit) fn checked(
        owner: (u64, Option<UsJurisdiction>),
        kind: PublishedRosterOwnerKindV1,
        span: SourceSpan,
        termination: PublishedRosterOwnerTerminationV1,
    ) -> Result<Self, RosterWitnessUnavailableV1> {
        if owner.0 == 0 {
            return Err(RosterWitnessUnavailableV1::MissingPublishedOwner);
        }
        Ok(Self {
            provider_id: owner.0,
            jurisdiction: owner.1,
            kind,
            span,
            termination,
        })
    }

    pub fn provider_id(&self) -> u64 {
        self.provider_id
    }
    pub fn jurisdiction(&self) -> Option<UsJurisdiction> {
        self.jurisdiction
    }
    pub fn kind(&self) -> PublishedRosterOwnerKindV1 {
        self.kind
    }
    pub fn span(&self) -> SourceSpan {
        self.span
    }
    pub fn termination(&self) -> PublishedRosterOwnerTerminationV1 {
        self.termination
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedRosterOwnersV1 {
    declarations: Vec<PublishedRosterOwnerV1>,
}

impl PublishedRosterOwnersV1 {
    pub(in crate::milesplit) fn checked(
        declarations: Vec<PublishedRosterOwnerV1>,
    ) -> Result<Self, RosterWitnessUnavailableV1> {
        if declarations.is_empty() {
            return Err(RosterWitnessUnavailableV1::MissingPublishedOwner);
        }
        if declarations.len() > 32 {
            return Err(RosterWitnessUnavailableV1::OwnerInventoryLimit);
        }
        Ok(Self { declarations })
    }

    pub fn declarations(&self) -> &[PublishedRosterOwnerV1] {
        &self.declarations
    }
}

pub(in crate::milesplit) fn checked_span(
    html: &str,
    span: Range<usize>,
) -> Result<SourceSpan, RosterWitnessUnavailableV1> {
    if html.get(span.clone()).is_none() || span.start >= span.end {
        return Err(RosterWitnessUnavailableV1::MissingSourceSpan);
    }
    SourceSpan::new(span.start, span.end).map_err(|_| RosterWitnessUnavailableV1::SourceSpanLimit)
}
