mod patterns;

use self::patterns::{
    nd_address_regex, nd_coop_regex, nd_enrollment_regex, nd_heading_regex, nd_link_regex,
    nd_row_regex, nd_staff_regex, nd_website_regex,
};
use super::parse::{clean_text, nonempty, split_person_names, strip_coop_note, without_comments};
use super::{ND_ADAPTER_ID, ND_SCHOOL_BASE};
use crate::CrawlResult;
use census_domain::model::{
    normalize_name, CanonicalSchool, Evidence, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NdSchoolRef {
    pub id: String,
    pub slug: String,
}

impl NdSchoolRef {
    pub fn url(&self) -> String {
        format!("{ND_SCHOOL_BASE}{}/{}", self.id, self.slug)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdStaffRole {
    pub label: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdOffering {
    pub label: String,
    pub coaches: Vec<String>,
    pub co_op: Option<String>,
}

pub fn parse_nd_school_refs(html: &str) -> CrawlResult<Vec<NdSchoolRef>> {
    let html = without_comments(html)?;
    let html: &str = &html;
    let mut members: Vec<NdSchoolRef> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for capture in nd_link_regex()?.captures_iter(html) {
        let (Some(id), Some(slug)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        if !seen.insert(id.as_str()) {
            continue;
        }
        members.push(NdSchoolRef {
            id: id.as_str().to_string(),
            slug: slug.as_str().to_string(),
        });
    }
    Ok(members)
}

fn nd_city(html: &str) -> CrawlResult<Option<String>> {
    let captured = nd_address_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1));
    let Some(raw) = captured else {
        return Ok(None);
    };
    let address = clean_text(raw.as_str())?;
    let cut = address.rfind(", ND").unwrap_or(address.len());
    let city = address
        .get(..cut)
        .and_then(|before| before.rsplit(',').next());
    Ok(city.and_then(nonempty))
}

fn nd_enrollment(html: &str) -> CrawlResult<Option<u32>> {
    let digits = nd_enrollment_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|digits| {
            digits
                .as_str()
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
        });
    Ok(digits.and_then(|digits| digits.parse().ok()))
}

pub fn parse_nd_school_page(
    html: &str,
    member: &NdSchoolRef,
    observed_on: &str,
) -> CrawlResult<Option<(CanonicalSchool, SchoolId)>> {
    let html = without_comments(html)?;
    let html: &str = &html;
    let heading = nd_heading_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1));
    let Some(heading) = heading else {
        return Ok(None);
    };
    let name = clean_text(heading.as_str())?;
    if name.is_empty() {
        return Ok(None);
    }

    let url = member.url();
    let (mut school, school_id) =
        CanonicalSchool::new(UsJurisdiction::NorthDakota, &name, normalize_name(&name));
    school.city = nd_city(html)?;
    school.enrollment = nd_enrollment(html)?;
    school.school_website = nd_website_regex()?
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|href| nonempty(href.as_str()));
    school.association = Some(ND_ADAPTER_ID.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ND_ADAPTER_ID.to_string(),
            },
            &member.id,
        )
        .with_url(&url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(ND_ADAPTER_ID, Some(url)),
        observed_on,
    ));
    Ok(Some((school, school_id)))
}

pub fn parse_nd_staff(html: &str) -> CrawlResult<Vec<NdStaffRole>> {
    let html = without_comments(html)?;
    let html: &str = &html;
    let mut roles: Vec<NdStaffRole> = Vec::new();
    for capture in nd_staff_regex()?.captures_iter(html) {
        let (Some(label), Some(name)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        let label = clean_text(label.as_str())?;
        let name = clean_text(name.as_str())?;
        if label.is_empty() || name.is_empty() {
            continue;
        }
        roles.push(NdStaffRole { label, name });
    }
    Ok(roles)
}

pub fn parse_nd_offerings(html: &str) -> CrawlResult<Vec<NdOffering>> {
    let html = without_comments(html)?;
    let html: &str = &html;
    let mut offerings: Vec<NdOffering> = Vec::new();
    for capture in nd_row_regex()?.captures_iter(html) {
        let (Some(label), Some(names)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        let label = clean_text(label.as_str())?;
        if label.is_empty() {
            continue;
        }
        let co_op = nd_co_op(&label)?;
        offerings.push(NdOffering {
            label: strip_coop_note(&label)?,
            coaches: split_person_names(&clean_text(names.as_str())?)?,
            co_op,
        });
    }
    Ok(offerings)
}

fn nd_co_op(label: &str) -> CrawlResult<Option<String>> {
    let captured = nd_coop_regex()?
        .captures(label)
        .and_then(|coop| coop.get(1));
    let Some(value) = captured else {
        return Ok(None);
    };
    Ok(nonempty(&clean_text(value.as_str())?))
}
