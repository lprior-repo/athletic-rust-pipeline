use census_domain::model::CanonicalAthlete;

#[derive(Default)]
pub(super) struct Profiles {
    pub(super) athletic_net: Option<String>,
    pub(super) milesplit: Option<String>,
    pub(super) other: Vec<String>,
}

pub(super) fn profiles_of(athlete: &CanonicalAthlete) -> Profiles {
    let mut profiles = Profiles::default();
    let mut seen: Vec<String> = Vec::new();
    let candidates = athlete.public_profile_urls.iter().cloned().chain(
        athlete
            .identities()
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
