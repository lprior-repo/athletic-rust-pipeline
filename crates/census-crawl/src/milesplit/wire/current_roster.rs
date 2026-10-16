use super::SourceRowLocator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentRosterAssertion {
    pub(in crate::milesplit) owner: String,
    pub(in crate::milesplit) member: String,
    pub(in crate::milesplit) row: SourceRowLocator,
    pub(in crate::milesplit) section: std::ops::Range<usize>,
    pub(in crate::milesplit) container: std::ops::Range<usize>,
    pub(in crate::milesplit) heading: std::ops::Range<usize>,
    pub(in crate::milesplit) heading_sha256: [u8; 32],
    pub(in crate::milesplit) sports: [bool; 3],
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
