use super::super::parse::{DirectorySchool, SchoolSummary};
use super::{absorb_summary, coach_entities, CoachEmission, SOURCE_ID};
use census_domain::model::{
    CanonicalSchool, Evidence, SchoolAddressError, SchoolId, SchoolPostalAddress, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_directory::{
    CityName, DirectoryError, PostalAddress, SourceLabel, StreetLine, ZipCode,
};
use census_domain::UsJurisdiction;

#[derive(Clone, Copy)]
pub(crate) struct Capture<'a> {
    pub url: &'a str,
    pub observed_on: &'a str,
    pub sha256: &'a str,
}

#[derive(Debug, thiserror::Error)]
pub(in super::super) enum PostalReview {
    #[error("malformed published postal component: {0}")]
    Component(#[from] DirectoryError),
    #[error("published postal state is not a jurisdiction; original value retained in capture")]
    State,
    #[error("published ZIP is malformed; original value retained in capture")]
    Zip,
    #[error("malformed published postal fields: {0}")]
    Fields(String),
    #[error("postal claim rejected: {0}")]
    Claim(#[from] SchoolAddressError),
    #[error("postal claim has no school jurisdiction or exact short code")]
    MissingOwner,
}

#[derive(Debug, thiserror::Error)]
pub(in super::super) enum SummaryError {
    #[error("summary id does not match a nonempty directory orgId; original values retained in captures")]
    Organization,
    #[error("summary shortCode does not match directory shortCode; original values retained in captures")]
    ShortCode,
    #[error("summary jurisdiction or exact source owner does not match directory school")]
    Association,
    #[error("coach mapping failed: {0}")]
    Coaches(#[from] crate::CrawlError),
}

pub(in super::super) struct SummaryEmission {
    pub emission: CoachEmission,
    pub postal_review: Option<PostalReview>,
}

struct PublishedAddress<'a> {
    line1: Option<&'a str>,
    line2: Option<&'a str>,
    city: Option<&'a str>,
    state: Option<&'a str>,
    zip: Option<&'a str>,
    issues: &'a [String],
}

pub(in super::super) fn retain_directory_postal(
    school: &mut CanonicalSchool,
    row: &DirectorySchool,
    capture: Capture<'_>,
) -> Result<(), PostalReview> {
    attach(
        school,
        row.short_code.as_deref(),
        PublishedAddress {
            line1: row.address.as_deref(),
            line2: row.address2.as_deref(),
            city: row.city.as_deref(),
            state: row.state_code.as_deref(),
            zip: row.zip.as_deref(),
            issues: &row.postal_issues,
        },
        capture,
    )
}

pub(in super::super) fn process_owned_summary(
    school: &mut CanonicalSchool,
    row: &DirectorySchool,
    summary: &SchoolSummary,
    school_id: &SchoolId,
    capture: Capture<'_>,
    school_year: SchoolYear,
) -> Result<SummaryEmission, SummaryError> {
    verify_association(school, row, summary)?;
    absorb_summary(school, summary, capture.url, capture.observed_on);
    let postal_review = attach(
        school,
        Some(&summary.short_code),
        PublishedAddress {
            line1: summary.address.address1.as_deref(),
            line2: summary.address.address2.as_deref(),
            city: summary.address.city.as_deref(),
            state: summary.address.state.as_deref(),
            zip: summary.address.zip.as_deref(),
            issues: &summary.address.postal_issues,
        },
        capture,
    )
    .err();
    let emission = coach_entities(
        summary,
        school_id,
        capture.url,
        capture.observed_on,
        school_year,
        capture.sha256,
    )?;
    Ok(SummaryEmission {
        emission,
        postal_review,
    })
}

fn verify_association(
    school: &CanonicalSchool,
    row: &DirectorySchool,
    summary: &SchoolSummary,
) -> Result<(), SummaryError> {
    if !row
        .org_id
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty() && id == summary.id)
    {
        return Err(SummaryError::Organization);
    }
    if !row
        .short_code
        .as_deref()
        .is_some_and(|code| !code.trim().is_empty() && code == summary.short_code)
    {
        return Err(SummaryError::ShortCode);
    }
    let state = school.state.ok_or(SummaryError::Association)?;
    let namespace = SourceNamespace::association_school(SOURCE_ID);
    if !school
        .source_identities
        .iter()
        .any(|owner| owner.namespace == namespace && owner.id == summary.short_code)
        || row.state_code.as_deref().and_then(UsJurisdiction::parse) != Some(state)
        || UsJurisdiction::parse(&summary.state_code) != Some(state)
        || summary
            .address
            .state
            .as_deref()
            .and_then(UsJurisdiction::parse)
            .is_some_and(|address| address != state)
    {
        return Err(SummaryError::Association);
    }
    Ok(())
}

fn attach(
    school: &mut CanonicalSchool,
    short_code: Option<&str>,
    raw: PublishedAddress<'_>,
    capture: Capture<'_>,
) -> Result<(), PostalReview> {
    if !raw.issues.is_empty() {
        return Err(PostalReview::Fields(raw.issues.join("; ")));
    }
    let line1 = populated(raw.line1).map(StreetLine::parse).transpose()?;
    let line2 = populated(raw.line2).map(StreetLine::parse).transpose()?;
    let city = populated(raw.city).map(CityName::parse).transpose()?;
    let zip = populated(raw.zip).map(parse_zip).transpose()?;
    let state = populated(raw.state)
        .map(|raw| UsJurisdiction::parse(raw).ok_or(PostalReview::State))
        .transpose()?;
    let Some(line1) = line1 else {
        return Ok(());
    };
    let Some(address) = PostalAddress::of(Some(line1), line2, city, state, zip) else {
        return Err(PostalReview::Claim(SchoolAddressError::EmptyAddress));
    };
    let owner = short_code
        .filter(|code| !code.trim().is_empty())
        .ok_or(PostalReview::MissingOwner)?;
    let state = school.state.ok_or(PostalReview::MissingOwner)?;
    let claim = SchoolPostalAddress::new(
        address,
        SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), owner)
            .with_url(capture.url.to_string()),
        SourceLabel::AthleticAssociation { state },
        Evidence::parsed(
            SourceRef::new(SOURCE_ID, Some(capture.url.to_string())),
            capture.observed_on,
        ),
        capture.sha256.to_string(),
    )?;
    school.add_postal_address(claim)?;
    Ok(())
}

fn populated(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn parse_zip(raw: &str) -> Result<ZipCode, PostalReview> {
    if raw.trim().len() > 10 {
        return Err(PostalReview::Zip);
    }
    ZipCode::parse(raw).map_err(|_| PostalReview::Zip)
}
