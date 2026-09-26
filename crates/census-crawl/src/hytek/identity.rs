//! The identity columns of a row whose own section header does not state them.
//!
//! A relay header names a `School` column and no athlete, so every row beneath it is read as one
//! school label. A capture that drops an individual event's own header leaves `Name  Year  School`
//! rows in that place, where the label then carries all three:
//!
//! ```text
//!  -- Donavin Bond              10 Nicolet                   FOUL        2
//! ```
//!
//! The reader below reads such a label back as the individual row it is, because no school label
//! holds a standalone grade between two runs of text. Anything the shape does not fit is left as
//! the school label the layout declared — a wrong identity is worse than a reported skip.

use census_domain::model::Grade;

use super::columns::{grade_from_token, looks_like_a_name};

/// The athlete, grade and school a school-only column holds when the row it was read from is an
/// individual row.
///
/// The run before the grade is the athlete, the grade is the published year, and the run after it
/// is the school. `None` when the label is not that shape: no grade token, two grade-shaped tokens
/// (the reader declines rather than guessing which one is the year), the grade as the first or the
/// last token, a leading run that does not read as a name, or a trailing run that still carries a
/// digit — a mark the layout left in the column rather than a school.
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
