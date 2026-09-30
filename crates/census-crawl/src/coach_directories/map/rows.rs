use super::persons::{person_name, sport_family};
use crate::coach_directories::nonempty;
use crate::coach_directories::parse::StaffMember;
use crate::coach_directories::SOURCE_ID;
use crate::{row_hygiene, CrawlResult};
use census_domain::model::{
    CanonicalCoach, Evidence, Gender, SchoolId, SourceIdentity, SourceNamespace, SourceRef, Sport,
};

#[derive(Debug, Clone)]
pub struct Row<'a> {
    pub member: &'a StaffMember,
    pub person: String,
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: census_domain::model::CoachRole,
}

#[derive(Debug, Clone)]
pub struct Claim {
    pub person: String,
    pub sport: &'static str,
    pub gender: Gender,
    pub role: census_domain::model::CoachRole,
}

pub struct RowDraft<'a> {
    pub member: &'a StaffMember,
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: census_domain::model::CoachRole,
    pub level: Option<&'a str>,
}

pub fn push_row<'a>(
    rows: &mut Vec<Row<'a>>,
    claims: &mut Vec<Claim>,
    counters: &mut super::CoachCounters,
    scope: super::EmissionScope,
    draft: RowDraft<'a>,
) -> CrawlResult<()> {
    let RowDraft {
        member,
        sport,
        gender,
        role,
        level,
    } = draft;
    let mut person = person_name(member);
    let family = sport_family(sport);
    if is_duplicate_claim(claims, &person, family, gender, role) {
        return Ok(());
    }
    claims.push(Claim {
        person: person.clone(),
        sport: family,
        gender,
        role,
    });
    if census_level_filter(scope, level, counters) {
        return Ok(());
    }
    if scope == super::EmissionScope::Census {
        let Some(sanitized) = row_hygiene::sanitize_person(&person)? else {
            counters.dropped_person = counters.dropped_person.saturating_add(1);
            return Ok(());
        };
        person = sanitized;
        let vendor_address = member
            .emails
            .first()
            .map(String::as_str)
            .and_then(nonempty)
            .is_some_and(|address| row_hygiene::is_vendor_contact(&address));
        if vendor_address {
            counters.dropped_vendor = counters.dropped_vendor.saturating_add(1);
            return Ok(());
        }
    }
    rows.push(Row {
        member,
        person,
        sport,
        gender,
        role,
    });
    Ok(())
}

fn is_duplicate_claim(
    claims: &[Claim],
    person: &str,
    sport_family: &str,
    gender: Gender,
    role: census_domain::model::CoachRole,
) -> bool {
    claims.iter().any(|claim| {
        claim.person == person
            && claim.sport == sport_family
            && claim.gender == gender
            && claim.role == role
    })
}

fn census_level_filter(
    scope: super::EmissionScope,
    level: Option<&str>,
    counters: &mut super::CoachCounters,
) -> bool {
    if scope == super::EmissionScope::Census && !row_hygiene::is_varsity_level(level) {
        let slot = counters
            .dropped_levels
            .entry(row_hygiene::level_label(level))
            .or_insert(0);
        *slot = slot.saturating_add(1);
        return true;
    }
    false
}

pub fn build_coach(
    row: &Row<'_>,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(
        school_id,
        row.person.as_str(),
        row.sport,
        row.gender,
        row.role,
    );
    if let Some(address) = row
        .member
        .emails
        .first()
        .map(String::as_str)
        .and_then(nonempty)
    {
        coach.set_published_email(&address);
    }
    if let Some(number) = row
        .member
        .tel
        .first()
        .and_then(|tel| tel.num.as_deref())
        .and_then(nonempty)
    {
        coach.phone = Some(number);
    }
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::association_school(SOURCE_ID),
            identity_key(row).as_str(),
        )
        .with_url(source_url.to_string()),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    coach
}

fn identity_key(row: &Row<'_>) -> String {
    let sport = sport_family(row.sport);
    let code = match row.member.amr_id.as_deref().and_then(nonempty) {
        Some(code) => code,
        None => row.person.clone(),
    };
    format!(
        "{code}:{sport}:{}:{}",
        row.role.stable_key(),
        row.gender.stable_key()
    )
}
