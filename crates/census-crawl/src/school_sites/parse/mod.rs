mod admission;
mod hits;
mod patterns;
mod rules;
mod text;
mod urls;

pub use hits::{analyse, clean_person, AdHit, CoachHit, Signals};
pub use patterns::{
    CONTEXT_MAX, CONTEXT_RADIUS, MAX_ANCHORS, MAX_TABLES, MAX_TABLE_ROWS, TITLE_MAX,
};
pub use rules::Rules;
pub use text::truncate;
pub use urls::{base_domain, rank_link, resolve_href};
