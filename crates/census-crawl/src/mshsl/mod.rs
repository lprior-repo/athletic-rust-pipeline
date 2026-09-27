mod collect;
mod map;
mod parse;
mod teams;
mod text;

use census_domain::UsJurisdiction;

pub use collect::collect;
pub use map::{
    ad_coaches, ad_role, coach_entities, provider_key, published_coach_email, school_domains,
    school_entities,
};
pub use parse::{
    listing_page_url, parse_admin_entries, parse_next_listing_page, parse_school_detail,
    parse_school_list, school_page_url, AdminEntry, SchoolDetail, SchoolListRow,
};
pub use teams::{
    coach_role, is_published_level, parse_coach_records, parse_team_nodes, select_team_nodes,
    team_sport, CoachRecord, TeamCoaches, TeamNode,
};
pub use text::{decode_cfemail, decode_cfemail_fragment, strip_honorific};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

const SOURCE_ID: &str = "mshsl";
const COACH_NAMESPACE: &str = "mshsl_team_coach";
pub const SCHOOL_LIST_URL: &str = "https://www.mshsl.org/schools";
pub const SCHOOL_URL_PREFIX: &str = "https://www.mshsl.org/schools/";
pub const TEAMS_VIEW_URL: &str = "https://www.mshsl.org/jsonapi/views/teams/list_school";
pub const COACH_API_PREFIX: &str = "https://www.mshsl.org/api/coaches/";
const MAX_LISTING_PAGES: usize = 64;
const MAX_TEAMS_PER_SCHOOL: usize = 4;
const MAX_CFEMAIL_HEX: usize = 512;

#[cfg(test)]
mod tests;
