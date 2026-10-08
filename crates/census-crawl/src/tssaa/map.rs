use census_domain::model::{
    normalize_name, published_email, CanonicalCoach, CanonicalSchool, CoachContactClaim,
    CoachContactProgram, CoachRole, CoachTenure, CoachTenureEvidence, Evidence,
    SchoolPostalAddress, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_directory::{DirectoryKey, SchoolDirectoryEntry, StateRecordId};
use census_domain::UsJurisdiction;
use serde_json::json;

use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};

use super::{parse, parse_school_page, CoachRow, SOURCE_ID};

pub(super) struct Emission {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
    pub issues: Vec<String>,
}

pub(super) fn school_id(row: &SchoolDirectoryEntry) -> CrawlResult<&StateRecordId> {
    match row.key() {
        DirectoryKey::StateRecord {
            state: UsJurisdiction::Tennessee,
            id,
        } => Ok(id),
        _ => Err(parse::artifact(
            "indexed row has no Tennessee association school owner",
        )),
    }
}

pub(super) fn map_capture(
    indexed: &SchoolDirectoryEntry,
    capture: &FetchOutcome,
) -> CrawlResult<Emission> {
    let id = school_id(indexed)?;
    verify_url(&capture.url, id)?;
    let text = std::str::from_utf8(&capture.body)
        .map_err(|error| parse::artifact(format!("detail capture is not UTF-8: {error}")))?;
    let read = parse_school_page(text, id)?;
    let row = read
        .school
        .entries()
        .first()
        .ok_or_else(|| parse::artifact("detail has no school"))?;
    if indexed.name() != row.name() {
        return Err(parse::artifact("detail owner differs from discovery index"));
    }
    emit_read(&read, row, id, capture)
}

fn emit_read(
    read: &super::SchoolRead,
    row: &SchoolDirectoryEntry,
    id: &StateRecordId,
    capture: &FetchOutcome,
) -> CrawlResult<Emission> {
    let school = map_school(row, id, capture, &read.addresses)?;
    let mut issues = Vec::new();
    read.school
        .skipped()
        .iter()
        .chain(
            read.school
                .notes()
                .iter()
                .filter(|issue| issue.field == "email"),
        )
        .try_for_each(|issue| push_issue(&mut issues, issue.render()))?;
    if read.staff_year.is_none() {
        push_issue(
            &mut issues,
            "published staff-year metadata is missing".to_string(),
        )?;
    }
    let coaches = map_staff(read, &school, id, capture, &mut issues)?;
    Ok(Emission {
        school,
        coaches,
        issues,
    })
}

fn map_staff(
    read: &super::SchoolRead,
    school: &CanonicalSchool,
    id: &StateRecordId,
    capture: &FetchOutcome,
    issues: &mut Vec<String>,
) -> CrawlResult<Vec<CanonicalCoach>> {
    read.coaches
        .iter()
        .try_fold(Vec::new(), |mut coaches, row| {
            let coach = map_coach(row, school, id, capture, read.staff_year.as_ref());
            if row.email.is_some() && !coach.has_published_email() {
                push_issue(
                    issues,
                    format!("{}: published email is invalid", row.person),
                )?;
            }
            coaches.try_reserve(1).map_err(|_| CrawlError::Resource {
                resource: "TSSAA projected staff",
                requested: 1,
                limit: 8192,
            })?;
            coaches.push(coach);
            Ok(coaches)
        })
}

fn push_issue(issues: &mut Vec<String>, issue: String) -> CrawlResult<()> {
    issues.try_reserve(1).map_err(|_| CrawlError::Resource {
        resource: "TSSAA projected issues",
        requested: 1,
        limit: 8192,
    })?;
    issues.push(issue);
    Ok(())
}

fn map_school(
    row: &SchoolDirectoryEntry,
    id: &StateRecordId,
    capture: &FetchOutcome,
    addresses: &[(String, census_domain::school_directory::PostalAddress)],
) -> CrawlResult<CanonicalSchool> {
    let name = row
        .name()
        .ok_or_else(|| parse::artifact("detail has no school name"))?
        .as_str();
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        name,
        normalize_name(name),
        row.address()
            .and_then(|address| address.city())
            .map(|city| city.as_str()),
    );
    let owner = SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), id.as_str())
        .with_url(capture.url.clone());
    school.source_identities.push(owner.clone());
    school.evidence.push(capture_evidence(capture));
    addresses.iter().try_for_each(|(label, address)| {
        let mut evidence = capture_evidence(capture);
        evidence.note =
            Some(json!({"address_kind": label, "capture": capture_note(capture)}).to_string());
        let claim = SchoolPostalAddress::new(
            address.clone(),
            owner.clone(),
            parse::source(),
            evidence,
            capture.content_digest.clone(),
        )
        .map_err(mapping_error)?;
        school.add_postal_address(claim).map_err(mapping_error)
    })?;
    Ok(school)
}

fn mapping_error(error: census_domain::model::SchoolAddressError) -> CrawlError {
    parse::artifact(format!("published postal claim: {error}"))
}

fn verify_url(raw: &str, id: &StateRecordId) -> CrawlResult<()> {
    let url = url::Url::parse(raw).map_err(|error| parse::artifact(error.to_string()))?;
    if url.scheme() != "https"
        || url.host_str() != Some("portal.tssaa.org")
        || url.path() != "/common/directory/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err(parse::artifact(
            "detail capture URL is outside the public directory",
        ));
    }
    let mut query = url.query_pairs();
    if query
        .next()
        .as_ref()
        .map(|(key, value)| (key.as_ref(), value.as_ref()))
        != Some(("id", id.as_str()))
        || query.next().is_some()
    {
        return Err(parse::artifact(
            "detail capture URL does not name exactly its indexed school ID",
        ));
    }
    Ok(())
}

fn capture_note(capture: &FetchOutcome) -> serde_json::Value {
    json!({"capture_url": capture.url, "sha256": capture.content_digest,
        "acquired_at": capture.fetched_at})
}

fn capture_evidence(capture: &FetchOutcome) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(capture_note(capture).to_string());
    evidence
}

fn map_coach(
    row: &CoachRow,
    school: &CanonicalSchool,
    id: &StateRecordId,
    capture: &FetchOutcome,
    staff_year: Option<&super::staff_year::PublishedStaffYear>,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(&school.id, &row.person, row.sport, row.gender, row.role);
    if let Some(email) = &row.email {
        coach.set_published_email(email);
    }
    coach.phone.clone_from(&row.phone);
    if let Some(staff) = &row.source_staff_id {
        coach.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::association_school(SOURCE_ID),
                format!("{}:staff:{staff}", id.as_str()),
            )
            .with_url(capture.url.clone()),
        );
    }
    let mut evidence = capture_evidence(capture);
    evidence.note = Some(
        json!({"capture": capture_note(capture), "school_id": id.as_str(),
        "staff_id": row.source_staff_id, "published_role": row.published_role,
        "published_sport": row.published_sport})
        .to_string(),
    );
    coach.evidence.push(evidence);
    if let Some(tenure) = published_tenure(&coach, row, capture, staff_year) {
        coach.tenure_evidence.push(tenure);
    }
    coach
}

fn published_tenure(
    coach: &CanonicalCoach,
    row: &CoachRow,
    capture: &FetchOutcome,
    staff_year: Option<&super::staff_year::PublishedStaffYear>,
) -> Option<CoachTenureEvidence> {
    let published = staff_year?;
    let program = match (&coach.role, &coach.sport) {
        (CoachRole::AthleticDirector, _) => Some(CoachContactProgram::SchoolAthletics),
        (CoachRole::HeadCoach | CoachRole::AssistantCoach, Some(sport)) => {
            Some(CoachContactProgram::Team {
                sport: *sport,
                gender: coach.gender,
            })
        }
        _ => None,
    };
    let program = program?;
    let mailbox = row
        .email
        .as_deref()
        .and_then(published_email)
        .map(|(address, _)| address);
    Some(CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: published.year,
        },
        source: SourceRef::new(SOURCE_ID, Some(capture.url.clone())),
        source_sha256: capture.content_digest.clone(),
        retrieved_at: capture.fetched_at.clone(),
        statement: published.statement.clone(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program,
            mailbox,
        }),
    })
}
