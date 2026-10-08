use census_domain::model::{CoachId, CoachTenureEvidence};

use crate::export::provenance::SelectedContact;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook) struct ContactProvenance {
    pub(in crate::workbook) observed_on: String,
    pub(in crate::workbook) source_url: String,
    pub(in crate::workbook) source_sha256: String,
    pub(in crate::workbook) coach_id: CoachId,
}

impl ContactProvenance {
    pub(super) fn mailbox(selected: SelectedContact<'_>) -> Self {
        Self::of(&selected.claim().coach, selected.tenure())
    }

    pub(super) fn of(coach_id: &CoachId, fact: &CoachTenureEvidence) -> Self {
        Self {
            coach_id: coach_id.clone(),
            source_url: fact
                .source
                .url
                .clone()
                .map_or(String::new(), core::convert::identity),
            source_sha256: fact.source_sha256.clone(),
            observed_on: fact.retrieved_at.clone(),
        }
    }

    fn assign(&mut self, coach_id: &CoachId, fact: &CoachTenureEvidence) {
        if &self.coach_id != coach_id {
            self.coach_id.clone_from(coach_id);
        }
        assign_text(
            &mut self.source_url,
            fact.source.url.as_deref().map_or("", |url| url),
        );
        assign_text(&mut self.source_sha256, &fact.source_sha256);
        assign_text(&mut self.observed_on, &fact.retrieved_at);
    }
}

fn assign_text(current: &mut String, next: &str) {
    if current.as_str() != next {
        current.clear();
        current.push_str(next);
    }
}

pub(super) fn merge(
    current: &mut Option<ContactProvenance>,
    coach_id: &CoachId,
    fact: &CoachTenureEvidence,
) {
    let next = (
        fact.retrieved_at.as_str(),
        fact.source.url.as_deref().map_or("", |url| url),
        fact.source_sha256.as_str(),
        coach_id,
    );
    if current.as_ref().is_none_or(|current| next > key(current)) {
        match current {
            Some(current) => current.assign(coach_id, fact),
            None => *current = Some(ContactProvenance::of(coach_id, fact)),
        }
    }
}

fn key(source: &ContactProvenance) -> (&str, &str, &str, &CoachId) {
    (
        &source.observed_on,
        &source.source_url,
        &source.source_sha256,
        &source.coach_id,
    )
}
