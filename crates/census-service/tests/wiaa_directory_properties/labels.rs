//! The labels the directory publishes, and what they are allowed to decide.
//!
//! Every sport, coaching role and administration role WIAA stores arrives as a printed label
//! (`Boys Track and Field`, `Assistant Coach`, `AD Admin Assistant`), and the same label is printed
//! `HEAD COACH` in one cell and `  Head Coach ` in the next. The case and the padding of a label are
//! a stylesheet's business; what the label means is a fact about the school, so both mappers work
//! from the label's words. The same holds for a person's name: `Coach Smith` and `Smith` are one
//! coach, not two.

use super::seam_config;
use census_crawl::wiaa::{parse_admin_role, parse_coach_role, parse_sport_label, strip_honorific};
use proptest::prelude::*;

/// Labels the directory actually publishes, one family per mapping: sports from `#tblCoachList`,
/// coaching roles from its fourth cell, administration roles from `#tblAdminList` — including the
/// office and secretarial roles that mention a director without being one.
const LABELS: [&str; 12] = [
    "Boys Track and Field",
    "Girls Cross Country",
    "Coed Track & Field",
    "Boys Cross-Country",
    "Head Coach",
    "Assistant Coach",
    "Asst Coach",
    "Coach",
    "Athletic Director",
    "Activities Director",
    "AD Admin Assistant",
    "Athletic Director Secretary",
];

/// Honorifics the directory prints in front of a name, and the names behind them.
const HONORIFICS: [&str; 4] = ["Coach", "Mr.", "Mrs", "Dr."];
const PEOPLE: [&str; 4] = ["Smith", "Van Der Berg", "O'Brien", "Lee"];

/// The same label the way another install of the page prints it: upper case, lower case, or padded
/// with the blanks a table cell carries.
fn restyle(label: &str, style: usize) -> String {
    match style {
        1 => label.to_ascii_uppercase(),
        2 => label.to_ascii_lowercase(),
        3 => format!(" \t{label}\n"),
        _ => label.to_string(),
    }
}

/// The pool reaches all three mappers in both directions: a property over labels that no page prints
/// would pass without ever touching a mapper.
#[test]
fn the_label_pool_reaches_every_mapper() {
    assert!(LABELS
        .iter()
        .any(|label| parse_sport_label(label).is_some()));
    assert!(LABELS
        .iter()
        .any(|label| parse_sport_label(label).is_none()));
    assert!(LABELS.iter().any(|label| parse_coach_role(label).is_some()));
    assert!(LABELS.iter().any(|label| parse_coach_role(label).is_none()));
    assert!(LABELS.iter().any(|label| parse_admin_role(label).is_some()));
    assert!(LABELS.iter().any(|label| parse_admin_role(label).is_none()));
}

proptest! {
    #![proptest_config(seam_config())]

    /// A label is read from its words: the case it is printed in and the blanks that pad it decide
    /// nothing about the sport, the gender or the role it means.
    #[test]
    fn a_label_is_read_from_its_words_not_their_case_or_blanks(
        label in prop::sample::select(Vec::from(LABELS)),
        style in 0usize..4,
    ) {
        let styled = restyle(label, style);
        prop_assert_eq!(
            parse_sport_label(&styled),
            parse_sport_label(label),
            "sport of {:?} printed as {:?}",
            label,
            styled
        );
        prop_assert_eq!(
            parse_coach_role(&styled),
            parse_coach_role(label),
            "coach role of {:?} printed as {:?}",
            label,
            styled
        );
        prop_assert_eq!(
            parse_admin_role(&styled),
            parse_admin_role(label),
            "administration role of {:?} printed as {:?}",
            label,
            styled
        );
    }

    /// An honorific is not part of the identity: `Coach Smith`, `COACH  Smith` and `Smith` are one
    /// coach, whatever case the honorific is printed in and whatever blank runs separate it.
    #[test]
    fn an_honorific_is_not_part_of_the_identity(
        honorific in prop::sample::select(Vec::from(HONORIFICS)),
        person in prop::sample::select(Vec::from(PEOPLE)),
        style in 0usize..3,
    ) {
        let written = match style {
            0 => format!("{honorific} {person}"),
            1 => format!("{honorific}  {person}"),
            _ => format!(" {} {}", honorific.to_ascii_uppercase(), person),
        };
        prop_assert_eq!(
            strip_honorific(&written),
            person.to_string(),
            "`{}` names `{}`",
            written,
            person
        );
        prop_assert_eq!(
            strip_honorific(person),
            person.to_string(),
            "a name without an honorific is left alone"
        );
    }
}
