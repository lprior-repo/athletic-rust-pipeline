//! Section-level formatting helpers for the census document.
//!
//! Each function returns a single Markdown string (heading + body) so the
//! caller can append it verbatim to the document buffer.

use super::{fmt_comma, fmt_pct};

/// Convert a usize to f64 safely; uses f64::MAX on overflow.
fn to_f64(val: usize) -> f64 {
    u32::try_from(val).map_or(f64::MAX, f64::from)
}

/// Build the "Totals" table body (without the heading).
pub(super) fn totals_body(totals: &super::data_loader::Totals) -> String {
    let total_schools = totals.total_schools;
    let total_athletes = totals.total_athletes;
    let total_co2027 = totals.total_co2027;
    let total_co2027_boys = totals.total_co2027_boys;
    let total_co2027_girls = totals.total_co2027_girls;
    let total_co2027_profile = totals.total_co2027_profile;
    let total_co2027_grade = totals.total_co2027_grade;
    let total_co2027_multi = totals.total_co2027_multi;
    let total_coaches = totals.total_coaches;
    let total_coaches_email = totals.total_coaches_email;
    let meets_total = totals.meets_total;
    let meets_an = totals.meets_an;
    format!(
        "| schools | {total_schools} |\n\
         | athletes (all grades) | {total_athletes} |\n\
         | Class of 2027 | {total_co2027} |\n\
         | Co2027 boys | {total_co2027_boys} |\n\
         | Co2027 girls | {total_co2027_girls} |\n\
         | Co2027 with a public profile URL | {total_co2027_profile} |\n\
         | Co2027 with grade evidence | {total_co2027_grade} |\n\
         | Co2027 reachable from two independent sources | {total_co2027_multi} |\n\
         | coaches / ADs | {total_coaches} |\n\
         | coaches / ADs with a published email | {total_coaches_email} |\n\
         | meets | {meets_total} |\n\
         | meets carrying an Athletic.net meet id | {meets_an} |"
    )
}

/// Build the "Athletic.net identities" bullet section.
pub(super) fn athletic_net_identities(
    seeds: &crate::cli::census_doc::counted_seeds::Seeds,
    co2027_len: usize,
    with_an: usize,
    with_ms: usize,
    multi_rows: usize,
) -> String {
    format!(
        "- **{} distinct Athletic.net athlete ids** were published by non-Athletic.net sources (AthleticLIVE timer rows that carry the `ani` field; MileSplit pages where the Athletic.net link is embedded). Each id yields the profile URL by composition: `https://www.athletic.net/athlete/<id>/track-and-field`.\n\
         - **{} distinct Athletic.net meet ids** were published by the timer meet index (`data/athleticnet-meet-seeds.csv`), so a targeted meet pull needs no search.\n\
         - {} canonical athletes are corroborated by more than one independent source (different namespaces on the same canonical id).\n\
         - Of the {} Co2027 rows in `data/canonical-athletes-co2027.csv`: {} carry an Athletic.net athlete id, {} carry a MileSplit athlete id, {} carry both kinds of identity.\n",
        fmt_comma(seeds.distinct_athletic_net_athlete_ids),
        fmt_comma(seeds.distinct_athletic_net_meet_ids),
        fmt_comma(seeds.multisource),
        fmt_comma(co2027_len),
        fmt_comma(with_an),
        fmt_comma(with_ms),
        fmt_comma(multi_rows),
    )
}

/// Build the "Coach coverage" bullet.
pub(super) fn coach_coverage_bullet(
    recruiting_len: usize,
    rec_coach: usize,
    rec_email: usize,
    rec_ad: usize,
) -> String {
    format!(
        "- Recruiting projection (`data/recruiting-co2027.csv`): {} Co2027 athletes, {} linked to a named head track/XC coach, {} with a published head-coach email, {} with an athletic-director email.\n",
        fmt_comma(recruiting_len),
        fmt_comma(rec_coach),
        fmt_comma(rec_email),
        fmt_comma(rec_ad),
    )
}

/// Format one measured-answer row.
fn answer_row(question: &str, answer: &str) -> String {
    format!("| {question} | {answer} |")
}

/// Build the measured answers table body for Q3 (share with AN profile).
fn q3_row(with_an: usize, co2027_len: usize) -> String {
    if co2027_len > 0 {
        answer_row(
            "Q3 share with an Athletic.net profile",
            &format!(
                "{} of {} Co2027 rows carry an Athletic.net id ({})",
                fmt_comma(with_an),
                fmt_comma(co2027_len),
                fmt_pct(to_f64(with_an), to_f64(co2027_len)),
            ),
        )
    } else {
        answer_row("Q3", "n/a")
    }
}

/// Build the measured answers table body for Q5 (share with identified coach).
fn q5_row(rec_coach: usize, recruiting_len: usize) -> String {
    if recruiting_len > 0 {
        answer_row(
            "Q5 share with an identified coach",
            &format!(
                "{} of {} Co2027 athletes ({})",
                fmt_comma(rec_coach),
                fmt_comma(recruiting_len),
                fmt_pct(to_f64(rec_coach), to_f64(recruiting_len)),
            ),
        )
    } else {
        answer_row("Q5", "n/a")
    }
}

/// Build the measured answers table body for Q6 (share with coach/AD email).
fn q6_row(rec_email: usize, rec_ad: usize, recruiting_len: usize) -> String {
    if recruiting_len > 0 {
        answer_row(
            "Q6 share with a public coach email",
            &format!(
                "{} of {} ({}); AD email {} ({})",
                fmt_comma(rec_email),
                fmt_comma(recruiting_len),
                fmt_pct(to_f64(rec_email), to_f64(recruiting_len)),
                fmt_comma(rec_ad),
                fmt_pct(to_f64(rec_ad), to_f64(recruiting_len)),
            ),
        )
    } else {
        answer_row("Q6", "n/a")
    }
}

/// The counts the "Measured answers" table reports alongside the counted seeds.
pub(super) struct MeasuredCounts {
    pub(super) total_co2027: u64,
    pub(super) total_athletes: u64,
    pub(super) co2027_len: usize,
    pub(super) with_an: usize,
    pub(super) recruiting_len: usize,
    pub(super) rec_coach: usize,
    pub(super) rec_email: usize,
    pub(super) rec_ad: usize,
}

/// Build the "Measured answers" Q1–Q9 table body.
pub(super) fn measured_answers(
    seeds: &crate::cli::census_doc::counted_seeds::Seeds,
    counts: &MeasuredCounts,
) -> String {
    let co2027_usize = usize::try_from(counts.total_co2027).unwrap_or(usize::MAX);
    let athletes_usize = usize::try_from(counts.total_athletes).unwrap_or(usize::MAX);

    [
        answer_row(
            "Q1 how many Co2027 athletes each source discovers",
            &format!(
                "see the by-state table; AthleticLIVE and MileSplit contribute {} canonical athletes across all grades, {} of them Co2027",
                fmt_comma(seeds.athletes),
                fmt_comma(co2027_usize),
            ),
        ),
        answer_row(
            "Q2 unique athletes after reconciliation",
            &format!(
                "{} canonical athletes minted from (school, name, class, gender) — no vendor id required",
                fmt_comma(athletes_usize),
            ),
        ),
        q3_row(counts.with_an, counts.co2027_len),
        answer_row(
            "Q4 share with independent corroboration",
            &format!(
                "{} athletes carry identities from two independent namespaces",
                fmt_comma(seeds.multisource),
            ),
        ),
        q5_row(counts.rec_coach, counts.recruiting_len),
        q6_row(counts.rec_email, counts.rec_ad, counts.recruiting_len),
        answer_row(
            "Q7 Athletic.net requests avoided",
            &format!(
                "{} athlete profiles and {} meet resources are addressable by id without any Athletic.net search or enumeration",
                fmt_comma(seeds.distinct_athletic_net_athlete_ids),
                fmt_comma(seeds.distinct_athletic_net_meet_ids),
            ),
        ),
        answer_row(
            "Q8 remaining gaps",
            "ND and SD MileSplit team indexes answered HTTP 403 and were not bypassed; MO/IN coach contact remains unpublished; see `synthesis/05-compliance-and-risks.md`",
        ),
        answer_row(
            "Q9 best adapters",
            "`data/adapter-ranking.csv`, now with measured yields substituted where the adapter was run",
        ),
    ]
    .join("\n")
}

/// Build the "Limits and honest gaps" bullet section.
pub(super) fn limits() -> String {
    [
        "The measurement covers what the runbooks in `tools/run_pipeline.sh` executed on 2026-09-20; it is a snapshot, not a season-long census.".to_string(),
        "Team rosters only surface athletes whose school publishes a roster on the source used; athletes on teams with no published roster are invisible to this pass.".to_string(),
        "`events` and `performances` tables are intentionally empty in this store: results acquisition stays with the production pipeline; this store carries identity, school, coach and meet facts.".to_string(),
    ]
    .join("\n- ")
}

/// Build the "Reproduce" code block.
pub(super) fn reproduce() -> String {
    [
        "```bash".to_string(),
        "# 1. crawl + adapters + merge + census + exports (idempotent, journal-resumed)"
            .to_string(),
        "tools/run_pipeline.sh".to_string(),
        "# 2. rebuild this document from the fresh snapshots".to_string(),
        "python3 tools/make_census_doc.py".to_string(),
        "```".to_string(),
    ]
    .join("\n")
}
