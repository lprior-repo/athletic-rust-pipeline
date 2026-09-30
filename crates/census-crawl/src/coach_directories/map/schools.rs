use crate::coach_directories::parse::{DirectorySchool, SchoolSummary};
use crate::coach_directories::{classification, nonempty, SOURCE_ID};
use crate::{row_hygiene, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalSchool, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryAdmission {
    School(Box<CanonicalSchool>, SchoolId),
    MissingShortCode,
    DroppedName,
}

pub fn directory_school(
    state: UsJurisdiction,
    association: &str,
    row: &DirectorySchool,
    source_url: &str,
    observed_on: &str,
) -> CrawlResult<DirectoryAdmission> {
    let Some(short_code) = row.short_code.as_deref().and_then(nonempty) else {
        return Ok(DirectoryAdmission::MissingShortCode);
    };
    let Some(name) = row_hygiene::sanitize_school(row.name.as_deref().unwrap_or_default())? else {
        return Ok(DirectoryAdmission::DroppedName);
    };
    let (mut school, id) = CanonicalSchool::new(state, name.as_str(), normalize_name(&name));
    school.city = row.city.as_deref().and_then(nonempty);
    school.association = Some(association.to_string());
    school.classification = classification(&row.competition_levels);
    apply_school_source_identity(&mut school, short_code.as_str(), source_url, observed_on);
    Ok(DirectoryAdmission::School(Box::new(school), id))
}

fn apply_school_source_identity(
    school: &mut CanonicalSchool,
    short_code: &str,
    source_url: &str,
    observed_on: &str,
) {
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), short_code)
            .with_url(source_url.to_string()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
}

pub fn absorb_summary(
    school: &mut CanonicalSchool,
    summary: &SchoolSummary,
    source_url: &str,
    observed_on: &str,
) {
    if let Some(name) = nonempty(&summary.name) {
        if name != school.name && !school.aliases.iter().any(|alias| alias == &name) {
            school.aliases.push(name);
        }
    }
    if school.city.is_none() {
        school.city = summary.address.city.as_deref().and_then(nonempty);
    }
    if school.classification.is_none() {
        school.classification = classification(&summary.competition_levels);
    }
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
}
