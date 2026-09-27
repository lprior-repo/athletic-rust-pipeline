
mod policy;
mod table;

use std::num::NonZeroUsize;

pub use table::descriptors;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    StructuredApi,
    StaticJson,
    Csv,
    Xml,
    Xlsx,
    Html,
    Pdf,
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilities {
    pub athlete_discovery: bool,
    pub athlete_profile: bool,
    pub meet_discovery: bool,
    pub bulk_results: bool,
    pub grade_evidence: bool,
    pub graduation_evidence: bool,
    pub school_evidence: bool,
    pub coach_directory: bool,
    pub public_professional_contact: bool,
    pub pr_evidence: bool,
}

impl SourceCapabilities {
    const NONE: SourceCapabilities = SourceCapabilities {
        athlete_discovery: false,
        athlete_profile: false,
        meet_discovery: false,
        bulk_results: false,
        grade_evidence: false,
        graduation_evidence: false,
        school_evidence: false,
        coach_directory: false,
        public_professional_contact: false,
        pr_evidence: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceAdmission {
    pub origin: &'static str,
    pub target_requests_per_second: f64,
    pub maximum_in_flight: NonZeroUsize,
    pub robots_crawl_delay_respected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceDescriptor {
    pub slug: &'static str,
    pub provider: &'static str,
    pub transport: TransportKind,
    pub capabilities: SourceCapabilities,
    pub admission: SourceAdmission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessClass {
    Open,
    Artifact,
    BrowserSession,
}

impl AccessClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Artifact => "artifact",
            Self::BrowserSession => "browser_session",
        }
    }
}

impl SourceDescriptor {
    pub fn access_class(&self) -> AccessClass {
        if self.admission.origin == policy::ARTIFACT_ORIGIN {
            AccessClass::Artifact
        } else if self.transport == TransportKind::Browser {
            AccessClass::BrowserSession
        } else {
            AccessClass::Open
        }
    }
}

pub fn descriptor(slug: &str) -> Option<&'static SourceDescriptor> {
    descriptors().find(|entry| entry.slug == slug)
}

pub fn transport_for_host(host: &str) -> Option<TransportKind> {
    descriptors()
        .find(|entry| entry.admission.origin.eq_ignore_ascii_case(host))
        .map(|entry| entry.transport)
}

pub fn bulk_first(slugs: &[&str]) -> Vec<&'static SourceDescriptor> {
    let mut ordered: Vec<&'static SourceDescriptor> =
        slugs.iter().copied().filter_map(descriptor).collect();
    ordered.sort_by_key(|entry| (planning_band(&entry.capabilities), entry.slug));
    ordered.dedup_by_key(|entry| entry.slug);
    ordered
}

fn planning_band(capabilities: &SourceCapabilities) -> u8 {
    if capabilities.bulk_results {
        0
    } else if capabilities.meet_discovery {
        1
    } else if capabilities.athlete_discovery || capabilities.athlete_profile {
        2
    } else {
        3
    }
}

#[cfg(test)]
#[path = "registry/tests.rs"]
mod tests;
