//! The `Athletes` sheet's three profile-URL columns, split out of the sheet module to keep both
//! files inside the size budget.
//!
//! A stored athlete carries public profile URLs from the sources that placed them, plus the URLs of
//! its own source identities. The sheet publishes them in three columns — `Athletic.net URL`,
//! `MileSplit URL`, `Other profile URLs` — and this module is the whole rule: the first URL for each
//! publication host wins a host column, everything else is kept in stored order, and a URL the
//! athlete stores twice is published once.

use census_domain::model::CanonicalAthlete;

/// The athlete's public profile URLs, split into the sheet's three columns.
#[derive(Default)]
pub(super) struct Profiles {
    pub(super) athletic_net: Option<String>,
    pub(super) milesplit: Option<String>,
    pub(super) other: Vec<String>,
}

/// The athlete's stored profile URLs, de-duplicated in stored order: the profile URLs first, then
/// the source-identity URLs.
pub(super) fn profiles_of(athlete: &CanonicalAthlete) -> Profiles {
    let mut profiles = Profiles::default();
    let mut seen: Vec<String> = Vec::new();
    let candidates = athlete.public_profile_urls.iter().cloned().chain(
        athlete
            .source_identities
            .iter()
            .filter_map(|identity| identity.url.clone()),
    );
    for url in candidates {
        if url.is_empty() || seen.contains(&url) {
            continue;
        }
        seen.push(url.clone());
        place_url(&mut profiles, url);
    }
    profiles
}

/// File one URL under the column whose host it belongs to.
fn place_url(profiles: &mut Profiles, url: String) {
    let lowered = url.to_ascii_lowercase();
    if lowered.contains("athletic.net") {
        if profiles.athletic_net.is_none() {
            profiles.athletic_net = Some(url);
        }
    } else if lowered.contains("milesplit") {
        if profiles.milesplit.is_none() {
            profiles.milesplit = Some(url);
        }
    } else {
        profiles.other.push(url);
    }
}
