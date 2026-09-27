use super::seam_config;
use census_crawl::wiaa::{parse_admin_role, parse_coach_role, parse_sport_label, strip_honorific};
use proptest::prelude::*;

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

const HONORIFICS: [&str; 4] = ["Coach", "Mr.", "Mrs", "Dr."];
const PEOPLE: [&str; 4] = ["Smith", "Van Der Berg", "O'Brien", "Lee"];

fn restyle(label: &str, style: usize) -> String {
    match style {
        1 => label.to_ascii_uppercase(),
        2 => label.to_ascii_lowercase(),
        3 => format!(" \t{label}\n"),
        _ => label.to_string(),
    }
}

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
