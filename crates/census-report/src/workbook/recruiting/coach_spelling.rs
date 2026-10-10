use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::CanonicalCoach;

pub(in crate::workbook) const SHEET_ROWS: &str = "Coaches sheet rows";
pub(in crate::workbook) const DISTINCT_SCHOOLS: &str = "Distinct schools with a coach row";
pub(in crate::workbook) const DISTINCT_COACH_IDS: &str = "Distinct coach IDs";

pub(in crate::workbook) struct CoachSheetCensus {
    pub(in crate::workbook) rows: usize,
    pub(in crate::workbook) schools: usize,
    pub(in crate::workbook) coach_ids: usize,
}

pub(in crate::workbook) fn spellings(coaches: &[CanonicalCoach]) -> BTreeMap<String, String> {
    let mut grouped: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for coach in coaches {
        grouped
            .entry(coach.id.as_str())
            .or_default()
            .push(coach.name.as_str());
    }
    grouped
        .into_iter()
        .map(|(id, names)| (id.to_string(), choose(names).to_string()))
        .collect()
}

pub(in crate::workbook) fn published<'a>(
    spellings: &'a BTreeMap<String, String>,
    coach: &'a CanonicalCoach,
) -> &'a str {
    spellings
        .get(coach.id.as_str())
        .map(String::as_str)
        .unwrap_or(coach.name.as_str())
}

pub(in crate::workbook) fn coach_sheet_census(coaches: &[CanonicalCoach]) -> CoachSheetCensus {
    let mut schools = BTreeSet::new();
    let mut coach_ids = BTreeSet::new();
    for coach in coaches {
        schools.insert(coach.school.as_str());
        coach_ids.insert(coach.id.as_str());
    }
    CoachSheetCensus {
        rows: coaches.len(),
        schools: schools.len(),
        coach_ids: coach_ids.len(),
    }
}

fn choose(names: Vec<&str>) -> &str {
    let mut best = "";
    let mut seen = false;
    for name in names {
        if !seen || better(name, best) {
            best = name;
            seen = true;
        }
    }
    best
}

fn better(candidate: &str, current: &str) -> bool {
    match rank(candidate).cmp(&rank(current)) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => candidate < current,
    }
}

fn rank(name: &str) -> (i32, i32) {
    let mut title: i32 = 0;
    let mut penalty: i32 = 0;
    for token in name.split_whitespace() {
        match token_case(token) {
            TokenCase::Title => title = title.saturating_add(1),
            TokenCase::Shout | TokenCase::Lower => penalty = penalty.saturating_add(1),
            TokenCase::Other => {}
        }
    }
    (title, penalty.saturating_neg())
}

enum TokenCase {
    Title,
    Shout,
    Lower,
    Other,
}

fn token_case(token: &str) -> TokenCase {
    let word = token.trim_matches(|ch: char| matches!(ch, '.' | ','));
    let mut letters = word.chars().filter(|ch| ch.is_alphabetic());
    let Some(first) = letters.next() else {
        return TokenCase::Other;
    };
    let mut rest_upper = true;
    let mut rest_lower = true;
    let mut rest: usize = 0;
    for ch in letters {
        rest = rest.saturating_add(1);
        rest_upper &= ch.is_uppercase();
        rest_lower &= ch.is_lowercase();
    }
    if first.is_uppercase() && rest_lower {
        TokenCase::Title
    } else if first.is_uppercase() && rest_upper && rest > 0 {
        TokenCase::Shout
    } else if first.is_lowercase() && rest_lower && rest > 0 {
        TokenCase::Lower
    } else {
        TokenCase::Other
    }
}

#[cfg(test)]
mod tests {
    use census_domain::model::{CanonicalCoach, CoachRole, Gender, SchoolId};

    use super::{coach_sheet_census, spellings, DISTINCT_COACH_IDS, DISTINCT_SCHOOLS, SHEET_ROWS};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn coach(school: &SchoolId, name: &str) -> CanonicalCoach {
        CanonicalCoach::new(school, name, None, Gender::Boys, CoachRole::HeadCoach)
    }

    #[test]
    fn one_coach_id_publishes_one_observed_spelling() -> TestResult {
        let school = SchoolId::mint("sch", &["spelling"]);
        let pairs = [
            ("T. J. pugh", "T.j. Pugh", "T.j. Pugh"),
            ("RONALD Thacker", "Ronald Thacker", "Ronald Thacker"),
            ("kennadee pringle", "Kennadee Pringle", "Kennadee Pringle"),
            ("JAKE SUSIC", "Jake Susic", "Jake Susic"),
            ("Ivan Smith jr", "Ivan Smith, jr.", "Ivan Smith jr"),
        ];
        for (left_name, right_name, expected) in pairs {
            let left = coach(&school, left_name);
            let right = coach(&school, right_name);
            check!(eq; left.id, right.id);
            let chosen = spellings(&[left.clone(), right]);
            check!(eq; chosen.get(left.id.as_str()).map(String::as_str), Some(expected));
        }
        Ok(())
    }

    #[test]
    fn coaches_sheet_census_names_schools_apart_from_coach_ids() -> TestResult {
        let school = SchoolId::mint("sch", &["census"]);
        let repeated = coach(&school, "Ronald Thacker");
        let variant = coach(&school, "RONALD Thacker");
        let second = coach(&school, "Jake Susic");
        let census = coach_sheet_census(&[repeated, variant, second]);
        check!(eq; census.rows, 3);
        check!(eq; census.schools, 1);
        check!(eq; census.coach_ids, 2);
        check!(ne; census.schools, census.coach_ids);
        check!(ne; SHEET_ROWS, DISTINCT_SCHOOLS);
        check!(ne; DISTINCT_SCHOOLS, DISTINCT_COACH_IDS);
        check!(
            DISTINCT_SCHOOLS.contains("schools"),
            "school count must say schools"
        );
        check!(
            DISTINCT_COACH_IDS.contains("coach"),
            "coach-id count must say coach"
        );
        Ok(())
    }
}
