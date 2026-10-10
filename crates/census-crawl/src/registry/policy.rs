use super::SourceAdmission;
use super::SourceCapabilities as Caps;
use std::num::NonZeroUsize;

pub(super) const FETCHER_RPS: f64 = 1.0;

pub(super) const HOME_CAMPUS_RPS: f64 = 0.5;

pub(super) const CRAWL_DELAY_TEN_RPS: f64 = 0.1;

pub(super) const CRAWL_DELAY_THIRTY_RPS: f64 = 1.0 / 30.0;

pub(super) const ARTIFACT_ORIGIN: &str = "local-artifact";

pub(super) const SCHOOL_COACH_CONTACT: Caps = Caps {
    school_evidence: true,
    coach_directory: true,
    public_professional_contact: true,
    ..Caps::NONE
};

pub(super) const SCHOOL_COACH_NAMES: Caps = Caps {
    school_evidence: true,
    coach_directory: true,
    ..Caps::NONE
};

pub(super) const SCHOOL_ADDRESS: Caps = Caps {
    school_evidence: true,
    ..Caps::NONE
};

pub(super) const fn fetched(origin: &'static str, rps: f64) -> SourceAdmission {
    SourceAdmission {
        origin,
        target_requests_per_second: rps,
        maximum_in_flight: NonZeroUsize::MIN,
    }
}

pub(super) const fn artifact() -> SourceAdmission {
    fetched(ARTIFACT_ORIGIN, FETCHER_RPS)
}
