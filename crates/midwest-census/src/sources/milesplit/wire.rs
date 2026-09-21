//! The site registry and the HTML shapes a team index and a roster are read into.
use census_domain::model::{Gender, GradYear};

/// One MileSplit state site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    pub state: &'static str,
    pub host: &'static str,
}

pub const SITES: [Site; 12] = [
    Site {
        state: "WI",
        host: "wi.milesplit.com",
    },
    Site {
        state: "MN",
        host: "mn.milesplit.com",
    },
    Site {
        state: "IA",
        host: "ia.milesplit.com",
    },
    Site {
        state: "IL",
        host: "il.milesplit.com",
    },
    Site {
        state: "MI",
        host: "mi.milesplit.com",
    },
    Site {
        state: "IN",
        host: "in.milesplit.com",
    },
    Site {
        state: "OH",
        host: "oh.milesplit.com",
    },
    Site {
        state: "MO",
        host: "mo.milesplit.com",
    },
    Site {
        state: "KS",
        host: "ks.milesplit.com",
    },
    Site {
        state: "NE",
        host: "ne.milesplit.com",
    },
    Site {
        state: "ND",
        host: "nd.milesplit.com",
    },
    Site {
        state: "SD",
        host: "sd.milesplit.com",
    },
];

impl Site {
    pub fn for_state(code: &str) -> Option<Site> {
        SITES
            .iter()
            .copied()
            .find(|site| site.state.eq_ignore_ascii_case(code))
    }

    pub fn teams_url(&self) -> String {
        format!("https://{}/teams", self.host)
    }

    pub fn source_id(&self) -> String {
        format!("milesplit_{}", self.state.to_ascii_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamRef {
    pub id: String,
    pub slug: String,
    pub url: String,
    pub name: String,
    pub city_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterAthlete {
    pub roster_name: String,
    pub name: String,
    pub gender: Gender,
    pub grad_year: GradYear,
    pub athlete_id: String,
    pub profile_url: String,
    pub indoor: bool,
    pub outdoor: bool,
    pub xc: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roster {
    pub team: TeamRef,
    pub athletes: Vec<RosterAthlete>,
}

