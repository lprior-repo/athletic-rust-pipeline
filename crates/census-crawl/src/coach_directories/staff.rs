use super::map::Row;
use census_domain::model::{
    CanonicalCoach, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
};

pub(super) fn build_coach(
    row: Row<'_>,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(school_id, row.person, row.sport, row.gender, row.role);
    if let Some(email) = row.member.emails.first() {
        if let Some(address) = super::nonempty(email) {
            coach.set_published_email(&address);
        }
    }
    if let Some(number) = row
        .member
        .tel
        .first()
        .and_then(|tel| tel.num.as_deref())
        .and_then(super::nonempty)
    {
        coach.phone = Some(number);
    }
    if let Some(source_key) = row.member.amr_id.as_deref().and_then(super::nonempty) {
        coach.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::association_school(super::SOURCE_ID),
                source_key,
            )
            .with_url(source_url.to_string()),
        );
    }
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(super::SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    coach
}
