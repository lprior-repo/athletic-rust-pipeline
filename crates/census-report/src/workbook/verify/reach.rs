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
}

impl Reach {
    pub(super) fn of(expectations: &Expectations<'_>, athlete: &CanonicalAthlete) -> Self {
        let school = expectations.contacts.get(athlete.school.as_str());
        let contacts = contact::scoped(school, athlete);
        let preferred = contacts.preferred();
        Self {
            track_names: Value::optional(contacts.track_names().as_deref()),
            track_emails: Value::optional(contacts.track_emails().as_deref()),
            cross_country_name: Value::optional(
                contacts.cross_country().map(|coach| coach.name.as_str()),
            ),
            cross_country_email: Value::optional(
                contacts.cross_country().and_then(|coach| coach.address()),
            ),
            professional: Value::optional(contacts.professional_coach_email()),
            director_name: Value::optional(contacts.director().map(|coach| coach.name.as_str())),
            director_email: Value::optional(
                contacts.director().and_then(|coach| coach.email.as_deref()),
            ),
            all_emails: Value::text(contacts.all_emails()),
            preferred_name: Value::text(preferred.name),
            preferred_role: Value::text(preferred.role),
            preferred_email: Value::text(preferred.email),
            preferred_state: Value::text(preferred.state.as_str()),
        }
    }
}
