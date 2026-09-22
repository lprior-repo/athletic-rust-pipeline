//! The registry's admission policy: the spacing constants, the two shared capability shapes and the
//! two admission constructors every table entry is built from.
//!
//! These live beside the table rather than inside it so the table stays a table - one entry per
//! adapter, each naming the symbol its `true` capabilities rest on - and the policy stays one
//! reviewable block.

use super::SourceAdmission;
use super::SourceCapabilities as Caps;
use std::num::NonZeroUsize;

/// The spacing a source fetched through the shared fetcher is held to: `--delay-ms` defaults to
/// 1000 ms, which is half the collection's 2 rps ceiling.
pub(super) const FETCHER_RPS: f64 = 1.0;

/// `www.wayzataresults.com` publishes `Crawl-delay: 10` for `User-agent: *`, and the fetcher applies
/// a robots crawl-delay as a floor on the configured spacing — so the host is declared at 0.1 rps
/// rather than at the default.
pub(super) const CRAWL_DELAY_TEN_RPS: f64 = 0.1;

/// Origin recorded for an adapter that reads a checked-in research artifact: it issues no request, so
/// there is no host to pace and the declared ceiling is the one a live fetch of the same material
/// would inherit.
pub(super) const ARTIFACT_ORIGIN: &str = "local-artifact";

/// The association-directory shape: a member-school universe, a coach and athletic-director
/// directory, and the professional address the directory publishes for those roles.
pub(super) const SCHOOL_COACH_CONTACT: Caps = Caps {
    school_evidence: true,
    coach_directory: true,
    public_professional_contact: true,
    ..Caps::NONE
};

/// The name-only shape: a member-school universe plus coach and director names, with no address
/// layer at all, so no contact claim can leak out of a directory that publishes none.
pub(super) const SCHOOL_COACH_NAMES: Caps = Caps {
    school_evidence: true,
    coach_directory: true,
    ..Caps::NONE
};

/// Admission of one origin fetched through the shared fetcher: `rps` spacing, one request in flight,
/// robots crawl-delay raised over the configured spacing and never lowered under it.
pub(super) const fn fetched(origin: &'static str, rps: f64) -> SourceAdmission {
    SourceAdmission {
        origin,
        target_requests_per_second: rps,
        maximum_in_flight: NonZeroUsize::MIN,
        robots_crawl_delay_respected: true,
    }
}

/// Admission of an adapter that reads an artifact instead of contacting a host.
pub(super) const fn artifact() -> SourceAdmission {
    fetched(ARTIFACT_ORIGIN, FETCHER_RPS)
}
