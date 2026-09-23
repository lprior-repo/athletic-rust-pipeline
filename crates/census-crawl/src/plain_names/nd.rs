//! North Dakota (NDHSAA): member-school index, per-school page, staff and offering parsers.
//!
//! `GET https://ndhsaa.com/schools` lists all 169 member schools; one
//! `GET https://ndhsaa.com/schools/<id>/<slug>` per school carries the staff block
//! (Superintendent, Principal, Athletic/Activities Director …) and the
//! `Sport/Activity Offering | Coaches` table parsed here.

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

/// One member school as listed on the NDHSAA school index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NdSchoolRef {
    /// Numeric NDHSAA school id — stable across seasons; the slug is cosmetic.
    pub id: String,
    /// URL slug, e.g. `west-fargo-sheyenne`.
    pub slug: String,
}

impl NdSchoolRef {
    /// Canonical per-school page URL.
    pub fn url(&self) -> String {
        format!("{ND_SCHOOL_BASE}{}/{}", self.id, self.slug)
    }
}

/// A published staff line: `<p>Role: Name</p>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdStaffRole {
    /// Published role label, e.g. `Athletic Director`.
    pub label: String,
    /// Published person name, honorifics intact.
    pub name: String,
}

/// One row of the NDHSAA "Sport/Activity Offering | Coaches" table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdOffering {
    /// Sport label with the co-op annotation removed, e.g. `Boys' Cross Country`.
    pub label: String,
    /// Coach names as published: separators split, honorifics stripped, duplicates collapsed.
    pub coaches: Vec<String>,
    /// Co-op annotation from the sport cell, e.g. `Some("Mandan")`.
    pub co_op: Option<String>,
}

/// Parse the member-school index, keeping the first link per numeric id.
///
/// The index is server-rendered and paginates nothing: one GET yields all 169 schools. Both the
/// absolute (`https://ndhsaa.com/schools/…`) and the relative (`/schools/…`) link forms are handled.
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

/// City from the `Address:` line: `800 40th Ave E., West Fargo, ND 58078` → `West Fargo`.
///
/// The line is read whole and cut at the trailing `, ND …`, then the last comma segment before the
/// state is the city. Matching a bare `City, ND 12345` pattern against the raw HTML would instead
/// swallow part of the street (`… th Ave E., West Fargo`), because the street itself contains commas.
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

/// Enrolment from `Grades 9-12, 1399 students enrolled in 2025`.
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

/// Canonical school for one NDHSAA school page.
///
/// The school name is the page's single `<h1>`; a page without one (or without a usable name) yields
/// `None` rather than a school named after a URL.
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

/// Staff lines (`Superintendent`, `Principal`, `Athletic Director`, `Business Manager`, …).
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

/// The coach table: one [`NdOffering`] per published row, blank coach cells included.
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

/// Co-op annotation published inside a row label: `(Co-op: West Fargo Sheyenne)` → `West Fargo Sheyenne`.
fn nd_co_op(label: &str) -> CrawlResult<Option<String>> {
    let captured = nd_coop_regex()?
        .captures(label)
        .and_then(|coop| coop.get(1));
    let Some(value) = captured else {
        return Ok(None);
    };
    Ok(nonempty(&clean_text(value.as_str())?))
}
