//! Parsed pages → canonical entities.
//!
//! One writer per collection run holds the accumulator, the school index and the run's counts.
//! Two page shapes are absorbed:
//!
//! - **Performance lists** (`/lists/<id>/<slug>/<year>/<i|o>`, optionally `?year=<TOKEN>`): one
//!   `CanonicalPerformance` per published mark, plus the athlete, team, event, meet and school
//!   the row implies. A mark is dated by its row's meet date, so the grade a `Year` cell states
//!   lands in the school year that contains the mark.
//! - **Team pages** (`/teams/<tf|xc>/<slug>_<m|f>.html`): the roster's athletes, dated by the
//!   season the page's own season control states. A roster publishes no marks, so it contributes
//!   no performances; it is the head-count route a list's top-N cannot produce.
//!
//! Grade evidence precedence. The row's own `Year` column is the published fact and always wins;
//! the `?year=` the page was requested with is a second, weaker channel used *only* for rows
//! that print no year at all. A filter that disagrees with a printed year is counted and never
//! obeyed (see `row`), which is what keeps an `&year=SR` view of one cohort from being published
//! as the target cohort: the token decides, so those rows land in the class the column states.
//!
//! Jurisdiction. TFRRS publishes one instance per state, so the page's own host is the state
//! every school it names is minted in (`crate::tfrrs::parse::jurisdiction_of_url`); a
//! page whose host names no state is never absorbed, because a school key without a state would
//! merge same-named schools across states.
//!
//! Identity: the athlete's numeric route id becomes a `TfrrsAthlete` identity, the team's slug a
//! `TfrrsTeam` identity, and a meet route's id a `TfrrsMeet` identity, so a later DirectAthletics
//! crawl that links the same meet (`tfrrs.org/results/<id>/…`) joins on it. Canonical ids stay
//! the domain's own: (school, name, grad year, gender) for an athlete, (school, sport, gender,
//! school year) for a team, (state, date, name) for a meet.
//!
//! Relay rows are read but not absorbed: the host prints the members' surnames only, with no
//! class year anywhere in the row, so minting an athlete from a surname would invent an
//! identity. The run counts them instead of guessing, and the numeric relay-athlete ids stay
//! available on the individual rows each member also appears in.
//!
//! Layout: `state` the run's own types, `list`/`roster` one page shape each, `entity`/`meet` the
//! mints, `row` the per-row decisions.

mod entity;
mod list;
mod meet;
mod roster;
mod row;
mod state;

pub(super) use state::{Absorb, ListContext, Page, RosterContext, Stats};
