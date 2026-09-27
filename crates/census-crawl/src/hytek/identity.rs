
use census_domain::model::Grade;

use super::columns::{grade_from_token, looks_like_a_name};

pub(super) fn individual_identity(label: &str) -> Option<(String, Grade, String)> {
    let parts: Vec<&str> = label.split_whitespace().collect();
    let mut graded = parts
        .iter()
        .enumerate()
        .filter_map(|(index, part)| grade_from_token(part).map(|grade| (index, grade)));
    let (grade_index, grade) = graded.next()?;
    if graded.next().is_some() {
        return None;
    }
    let name = parts.get(..grade_index)?.join(" ");
    let school = parts.get(grade_index.saturating_add(1)..)?.join(" ");
    if !looks_like_a_name(&name) {
        return None;
    }
    if school.is_empty() || school.chars().any(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((name, grade, school))
}

#[cfg(test)]
mod tests;
