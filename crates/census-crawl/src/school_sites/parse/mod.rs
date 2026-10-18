mod admission;
mod hits;
mod patterns;
mod rules;
mod staff;
mod text;
mod urls;

pub use hits::{analyse, clean_person, AdHit, CoachHit, SchoolMailboxHit, Signals};
pub use patterns::{
    CONTEXT_MAX, CONTEXT_RADIUS, MAX_ANCHORS, MAX_TABLES, MAX_TABLE_ROWS, TITLE_MAX,
};
pub use rules::Rules;
pub use staff::StaffContactHit;
pub use text::truncate;
pub(super) use urls::document_url;
pub use urls::{base_domain, rank_link, resolve_href};

pub(crate) use patterns::HIDDEN_BLOCK;
