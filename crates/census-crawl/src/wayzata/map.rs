use census_domain::model::CompetitionLevel;
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::HashMap;

const VENUE_STATES: [(&str, UsJurisdiction); 34] = [
    ("university of minnesota", UsJurisdiction::Minnesota),
    ("macalester", UsJurisdiction::Minnesota),
    ("st. olaf", UsJurisdiction::Minnesota),
    ("st olaf", UsJurisdiction::Minnesota),
    ("carleton college", UsJurisdiction::Minnesota),
    ("hamline", UsJurisdiction::Minnesota),
    ("gustavus", UsJurisdiction::Minnesota),
    ("bethel university", UsJurisdiction::Minnesota),
    ("university of st. thomas", UsJurisdiction::Minnesota),
    ("minnesota state mankato", UsJurisdiction::Minnesota),
    ("bemidji state", UsJurisdiction::Minnesota),
    ("concordia college moorhead", UsJurisdiction::Minnesota),
    ("university of iowa", UsJurisdiction::Iowa),
    ("northern iowa", UsJurisdiction::Iowa),
    ("iowa state", UsJurisdiction::Iowa),
    ("wartburg", UsJurisdiction::Iowa),
    ("drake university", UsJurisdiction::Iowa),
    ("luther college", UsJurisdiction::Iowa),
    ("simpson college", UsJurisdiction::Iowa),
    ("coe college", UsJurisdiction::Iowa),
    ("central college", UsJurisdiction::Iowa),
    ("grinnell", UsJurisdiction::Iowa),
    ("cornell college", UsJurisdiction::Iowa),
    ("loras", UsJurisdiction::Iowa),
    ("buena vista university", UsJurisdiction::Iowa),
    ("dubuque", UsJurisdiction::Iowa),
    ("mount mercy", UsJurisdiction::Iowa),
    ("uw-", UsJurisdiction::Wisconsin),
    ("university of wisconsin", UsJurisdiction::Wisconsin),
    ("-la crosse", UsJurisdiction::Wisconsin),
    ("eau claire", UsJurisdiction::Wisconsin),
    ("oshkosh", UsJurisdiction::Wisconsin),
    ("stevens point", UsJurisdiction::Wisconsin),
    ("whitewater", UsJurisdiction::Wisconsin),
];

pub fn venue_state(location: &str) -> Option<UsJurisdiction> {
    let location = location.to_ascii_lowercase();
    VENUE_STATES
        .iter()
        .find(|(marker, _)| location.contains(marker))
        .map(|(_, state)| *state)
}

const REGION_STATES: [UsJurisdiction; 3] = [
    UsJurisdiction::Minnesota,
    UsJurisdiction::Iowa,
    UsJurisdiction::Wisconsin,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueResolution {
    Site(UsJurisdiction),
    School(UsJurisdiction),
    Unknown,
}

impl VenueResolution {
    pub const fn state(self) -> Option<UsJurisdiction> {
        match self {
            VenueResolution::Site(state) | VenueResolution::School(state) => Some(state),
            VenueResolution::Unknown => None,
        }
    }
}

pub fn venue_candidates(location: &str) -> Vec<String> {
    let trimmed = location.trim();
    let mut candidates = vec![trimmed.to_string()];
    for (suffix, expansion) in [
        (" H.S.", " High School"),
        (" H.S", " High School"),
        (" HS.", " High School"),
        (" HS", " High School"),
        (" Middle School", " Middle School"),
        (" School", " School"),
    ] {
        let Some(base) = trimmed.strip_suffix(suffix) else {
            continue;
        };
        let base = base.trim();
        if base.is_empty() {
            continue;
        }
        let expanded = format!("{base}{expansion}");
        if !candidates.contains(&expanded) {
            candidates.push(expanded);
        }
    }
    candidates
}

pub fn resolve_venue(
    index: &SchoolIndex,
    cache: &mut HashMap<String, VenueResolution>,
    location: &str,
) -> VenueResolution {
    if let Some(cached) = cache.get(location) {
        return *cached;
    }
    let resolution = match venue_state(location) {
        Some(state) => VenueResolution::Site(state),
        None => {
            let candidates = venue_candidates(location);
            let mut hits: Vec<UsJurisdiction> = REGION_STATES
                .iter()
                .copied()
                .filter(|state| {
                    candidates
                        .iter()
                        .any(|label| index.resolve(*state, label).is_some())
                })
                .collect();
            hits.dedup();
            match hits.as_slice() {
                [only] => VenueResolution::School(*only),
                _ => VenueResolution::Unknown,
            }
        }
    };
    cache.insert(location.to_string(), resolution);
    resolution
}

pub fn level_of(name: &str) -> CompetitionLevel {
    let name = name.to_ascii_lowercase();
    let has = |needle: &str| name.contains(needle);
    if has("state") {
        CompetitionLevel::State
    } else if has("sectional") || has("section ") {
        CompetitionLevel::Sectional
    } else if has("regional") {
        CompetitionLevel::Regional
    } else if has("conference") || has("conf.") {
        CompetitionLevel::Conference
    } else if has("district") {
        CompetitionLevel::District
    } else if has("national") {
        CompetitionLevel::National
    } else if has("dual") {
        CompetitionLevel::Dual
    } else if has("invitational") || has("invite") || has("relays") || has("classic") || has("meet")
    {
        CompetitionLevel::Invitational
    } else {
        CompetitionLevel::Unknown
    }
}
