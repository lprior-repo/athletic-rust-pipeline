use super::canonical::Value;
use super::expectations::Expectations;
use crate::workbook::recruiting::contact;
use census_domain::model::CanonicalAthlete;

pub(super) struct Reach {
    pub(super) track_names: Value,
    pub(super) track_emails: Value,
    pub(super) cross_country_name: Value,
    pub(super) cross_country_email: Value,
    pub(super) professional: Value,
    pub(super) director_name: Value,
    pub(super) director_email: Value,
    pub(super) all_emails: Value,
    pub(super) preferred_name: Value,
    pub(super) preferred_role: Value,
    pub(super) preferred_email: Value,
    pub(super) preferred_state: Value,
    pub(super) preferred_coach_id: Value,
    pub(super) preferred_source_url: Value,
    pub(super) preferred_capture_sha256: Value,
    pub(super) preferred_acquired_at: Value,
}

impl Reach {
    pub(super) fn of(expectations: &Expectations<'_>, athlete: &CanonicalAthlete) -> Self {
        let school = expectations.contacts.get(athlete.school.as_str());
        let contacts = contact::scoped(school, athlete)
            .with_research(expectations.contact_research(athlete.school.as_str()));
        Self::from_contacts(&contacts)
    }

    fn from_contacts(contacts: &contact::ScopedContacts<'_>) -> Self {
        let preferred = contacts.preferred();
        let [cross_country_name, cross_country_email] = cross_country_values(contacts);
        let [director_name, director_email] = director_values(contacts);
        Self {
            track_names: Value::optional(contacts.track_names().as_deref()),
            track_emails: Value::optional(contacts.track_emails().as_deref()),
            cross_country_name,
            cross_country_email,
            professional: Value::optional(contacts.professional_coach_email()),
            director_name,
            director_email,
            all_emails: Value::text(contacts.all_emails()),
            preferred_name: Value::text(preferred.name),
            preferred_role: Value::text(preferred.role),
            preferred_email: Value::text(preferred.email),
            preferred_state: Value::text(preferred.state.as_str()),
            preferred_coach_id: Value::text(preferred.coach_id),
            preferred_source_url: Value::text(preferred.source_url),
            preferred_capture_sha256: Value::text(preferred.source_sha256),
            preferred_acquired_at: Value::text(preferred.observed_on),
        }
    }
}

fn cross_country_values(contacts: &contact::ScopedContacts<'_>) -> [Value; 2] {
    [
        Value::optional(contacts.cross_country().map(|coach| coach.name.as_str())),
        Value::optional(contacts.cross_country().and_then(|coach| coach.address())),
    ]
}

fn director_values(contacts: &contact::ScopedContacts<'_>) -> [Value; 2] {
    [
        Value::optional(contacts.director().map(|coach| coach.name.as_str())),
        Value::optional(contacts.director().and_then(|coach| coach.email.as_deref())),
    ]
}
