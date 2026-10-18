use super::super::result_ownership::CheckedRetainedResultDocumentV1;
use crate::milesplit::SourceRowLocator;
use census_domain::model::{
    AthleteId, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventId, MeetId,
    PerformanceId, SchoolId,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ProposedHtmlResultTargetV1<'a> {
    pub locator: SourceRowLocator,
    pub subject_reference: &'a str,
    pub proposed_athlete: &'a AthleteId,
    pub proposed_school: &'a SchoolId,
    pub proposed_attachment_reference: &'a str,
    pub proposed_school_binding_reference: &'a str,
}

pub struct ProposedHtmlResultIdentityV1 {
    pub(super) performance: PerformanceId,
    pub(super) athlete: AthleteId,
    pub(super) meet: MeetId,
    pub(super) event: EventId,
    pub(super) date: String,
    pub(super) source_key: String,
}

impl ProposedHtmlResultIdentityV1 {
    pub fn performance_id(&self) -> &PerformanceId {
        &self.performance
    }
    pub fn athlete(&self) -> &AthleteId {
        &self.athlete
    }
    pub fn meet_id(&self) -> &MeetId {
        &self.meet
    }
    pub fn event_id(&self) -> &EventId {
        &self.event
    }
    pub fn date(&self) -> &str {
        &self.date
    }
    pub fn source_key(&self) -> &str {
        &self.source_key
    }
}

pub struct UnadmittedHtmlResultProjectionV1<'a, 'capture> {
    pub(super) document: &'a CheckedRetainedResultDocumentV1<'capture>,
    pub(super) meet: CanonicalMeet,
    pub(super) rows: Vec<UnadmittedHtmlResultRowV1>,
}

impl<'a, 'capture> UnadmittedHtmlResultProjectionV1<'a, 'capture> {
    pub fn document(&self) -> &'a CheckedRetainedResultDocumentV1<'capture> {
        self.document
    }
    pub fn proposed_meet(&self) -> &CanonicalMeet {
        &self.meet
    }
    pub fn rows(&self) -> &[UnadmittedHtmlResultRowV1] {
        &self.rows
    }
    pub fn into_proposed_entities(self) -> (CanonicalMeet, Vec<UnadmittedHtmlResultRowV1>) {
        (self.meet, self.rows)
    }
}

#[derive(Debug, Serialize)]
pub struct UnadmittedHtmlResultRowV1 {
    pub(super) locator: SourceRowLocator,
    pub(super) subject_reference: String,
    pub(super) proposed_attachment_reference: String,
    pub(super) proposed_school_binding_reference: String,
    pub(super) event: CanonicalEvent,
    pub(super) team: CanonicalTeam,
    pub(super) performance: CanonicalPerformance,
}

impl UnadmittedHtmlResultRowV1 {
    pub fn locator(&self) -> SourceRowLocator {
        self.locator
    }
    pub fn subject_reference(&self) -> &str {
        &self.subject_reference
    }
    pub fn proposed_attachment_reference(&self) -> &str {
        &self.proposed_attachment_reference
    }
    pub fn proposed_school_binding_reference(&self) -> &str {
        &self.proposed_school_binding_reference
    }
    pub fn proposed_event(&self) -> &CanonicalEvent {
        &self.event
    }
    pub fn proposed_team(&self) -> &CanonicalTeam {
        &self.team
    }
    pub fn proposed_performance(&self) -> &CanonicalPerformance {
        &self.performance
    }
    pub fn into_proposed_entities(self) -> (CanonicalEvent, CanonicalTeam, CanonicalPerformance) {
        (self.event, self.team, self.performance)
    }
}
