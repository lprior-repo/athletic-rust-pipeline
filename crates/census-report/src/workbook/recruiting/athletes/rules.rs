//! The `Athletes` sheet (objective §50): one row per canonical athlete in the run's cohort.
//!
//! Published columns, in this order: Athlete ID, Name, Gender, Graduation Year, Current Grade, State,
//! School, School City, the four sport flags (TF, XC, Indoor, Outdoor), the event list, the headline
//! PR summary, the nineteen supported PR columns, performance and meet counts, the school contact
//! ladder, School Athletics URL, the two GPA columns, profile URLs, and the audit columns.
//!
//! Every cell is a stored field or a documented rule over stored fields:
//!
//! * the sport flags come from `CanonicalAthlete::sports`, and each PR column carries the winning
//!   mark from the same reduction the `PRs` sheet publishes;
//! * `Observed Grade` is the grade of the most recently observed `ObservedGrade`, which is evidence
//!   (§3), never a re-derivation of the cohort;
//! * `Observed School Year` is the school year the newest grade observation was captured in,
//!   e.g. `"2025-26"` — so a recruiter can see when the evidence was captured and whether it
//!   is from the current year (the only year a grade is treated as the athlete's current standing).
//! * the coach columns come from the school's head coaches and athletic director in the coach table;
//! * `Coverage State` is `pr` when the athlete has a row on the `PRs` sheet, `performance` when the
//!   athlete has stored performances but no comparable PR (relay legs and unparsed marks), and
//!   `identity-only` when the athlete has no scoped performance at all;
//! * `Conflict Flag` is `yes` when a grade observation implies a different graduating class than the
//!   athlete's canonical one, or when one source namespace carries two different ids for the athlete —
//!   the two disagreements the deterministic merge cannot settle on its own;
//! * `Confidence` is `"high"`, `"medium"`, `"low"`, or `"unknown"` based on the
//!   athlete's identity confidence; never a raw number.
//! * `Review Status` is `"verified"` when confidence is high with no conflicts,
//!   `"review"` when confidence is low or conflicts exist, and `"unknown"` when
//!   the merge has not reached a definitive verdict — an unknown state must never
//!   print as verified.
//!
//! `row_for` publishes those cells as identity, participation and PR columns, school contacts,
//! the preferred-contact ladder, profile URLs, and audit columns, in the order listed by `HEADERS`.
//!
//! # Column rules
//!
//! * `TF` — `yes` when the athlete's sports include `IndoorTrack` or `OutdoorTrack`, blank otherwise.
//! * `Event list` — the athlete's distinct events with a stored performance, in the canonical event
//!   order the sheet's per-event columns already use, joined with `; `. Blank when none.
//! * `Headline PR summary` — the athlete's PRs in canonical event order, formatted `Event Mark`,
//!   joined with `; `, capped at 10 entries with a trailing `...` when more exist. Blank when none.
//! * `Coach Professional Email` — the head TF coach's professional email; when absent, the head
//!   XC coach's professional email; when that is absent, the preferred recruiting contact's email
//!   if that contact's role is a coaching role (`Head TF Coach`, `Head XC Coach`, or `Head Coach`);
//!   blank otherwise.
//! * `School Athletics URL` — the stored `athletics_website` for the athlete's school; blank when
//!   the store holds none.
//! * `Public Recruiting GPA` and `GPA Source` — always blank. `census-domain` has no GPA observation
//!   entity, objective §36 forbids inferring one, and a GPA may only appear next to a `GPA Source`
//!   naming where it came from.
//!
//! # Who to contact, and how the sheet says it
//!
//! `School City` is the city on the athlete's school row (`CanonicalSchool::city`), blank when that row
//! carries none. The school contact columns carry the named coach/director rows and their own
//! professional-or-personal addresses, followed by the all-address inventory.
//!
//! `Preferred Recruiting Contact`, `Preferred Contact Role`, `Preferred Contact Email` and
//! `Contact Coverage State` are the athlete-specific answer, and the whole rule lives in [`super::contact`]:
//! The ladder reads `professional_email` first and explicitly falls back to `personal_email` when the
//! professional field is absent; the coverage state says a published address was found either way.
//! 1. the head coach of the athlete's evidence-bearing sport — a track slot (`Head TF Coach`) for an
//!    athlete with stored indoor or outdoor track evidence, a cross-country slot (`Head XC Coach`)
//!    for an athlete whose only sport is cross country, and the track slot for an athlete that stores
//!    no sport; `Preferred Contact Role` names the slot, plus the side of the team when the coach's
//!    row was published for one side (`Head TF Coach (girls)`);
//! 2. else the school's other head-coach slot;
//! 3. else a head coach whose row carries no sport binding;
//! 4. else the athletic director.
//!
//! `Contact Coverage State` is the typed `ContactState` vocabulary and is never blank: a coach's published
//! address is `professional_coach_email`, the director's is `professional_ad_email`, a named contact
//! with no published address anywhere is `coach_name_only`, a school whose coach rows name neither a
//! head coach nor a director is `no_public_contact_found`, and a school with no coach row at all is
//! `contact_source_not_attempted`. A blank cell therefore never means "we did not look".
//!
//! Where a school's rows name more than one head coach of one sport, the athlete's own side of the
//! team is preferred (the source publishes `Boys` and `Girls` head coaches for one sport), and within
//! one side the row that published an address wins, newest `Evidence.observed_on` first, then by
//! coach name and id — so two head coaches with different addresses never leave the cell to chance.
//! An email cell carries the address its field holds, professional or personal; a blank means no source
//! published one. The `All Emails (school)` cell carries every coach-row address for that school.

/// The worksheet name, as objective §50 publishes it.
pub(crate) const TITLE: &str = "Athletes";

/// The sheet's column headers, in published order.
pub(crate) const HEADERS: [&str; 60] = [
    "Athlete ID",
    "Name",
    "Gender",
    "Graduation Year",
    "Observed Grade",
    "Observed School Year",
    "State",
    "School",
    "School ID",
    "School City",
    "TF",
    "XC",
    "Indoor",
    "Outdoor",
    "Event list",
    "Headline PR summary",
    "100m (s)",
    "200m (s)",
    "400m (s)",
    "800m (s)",
    "1600m (s)",
    "3200m (s)",
    "1 Mile (s)",
    "5000m (s)",
    "100m Hurdles (s)",
    "110m Hurdles (s)",
    "300m Hurdles (s)",
    "High Jump (m)",
    "Long Jump (m)",
    "Triple Jump (m)",
    "Pole Vault (m)",
    "Shot Put (m)",
    "Discus (m)",
    "Javelin (m)",
    "XC (s)",
    "Performance count",
    "Meet count",
    "Head TF Coach",
    "Head TF Coach Email",
    "Head XC Coach",
    "Head XC Coach Email",
    "Coach Professional Email",
    "Athletic Director",
    "AD Email",
    "School Athletics URL",
    "Public Recruiting GPA",
    "GPA Source",
    "All Emails (school)",
    "Preferred Recruiting Contact",
    "Preferred Contact Role",
    "Preferred Contact Email",
    "Contact Coverage State",
    "Athletic.net URL",
    "MileSplit URL",
    "Other profile URLs",
    "Sources Count",
    "Confidence",
    "Coverage State",
    "Conflict Flag",
    "Review Status",
];

/// Column widths, one per header.
pub(crate) const WIDTHS: [u16; 60] = [
    20, 26, 10, 16, 12, 14, 8, 30, 16, 20, 8, 8, 8, 8, 30, 30, 12, 12, 12, 12, 12, 12, 12, 18, 18,
    18, 16, 16, 16, 16, 16, 16, 16, 12, 16, 12, 24, 32, 24, 32, 24, 32, 24, 24, 24, 24, 48, 30, 24,
    32, 28, 36, 36, 40, 14, 18, 18, 14, 14, 14,
];

/// The 19 PR event keys in canonical order.
pub(crate) const PR_EVENTS: [&str; 19] = [
    "Track100m",
    "Track200m",
    "Track400m",
    "Track800m",
    "Track1600m",
    "Track3200m",
    "Track1Mile",
    "Track5000m",
    "Track100mHurdles",
    "Track110mHurdles",
    "Track300mHurdles",
    "HighJump",
    "LongJump",
    "TripleJump",
    "PoleVault",
    "ShotPut",
    "Discus",
    "Javelin",
    "CrossCountry",
];
