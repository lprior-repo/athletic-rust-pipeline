use super::super::*;

const GROUPS: &[&str] = &[
    "varsity",
    "jv",
    "open",
    "junior",
    "senior",
    "freshman",
    "freshmen",
    "sophomore",
];

pub(super) fn parse(tokens: &[&str]) -> Result<Option<CompetitionCategory>, SpecificationError> {
    let category = gender_category(tokens)?;
    Ok(match group_category(tokens)? {
        Some(group) => Some(published_category_label(category, group)),
        None => category,
    })
}

pub(super) fn is_descriptor(token: &str) -> bool {
    gender(token).is_some() || is_group(token)
}

fn gender_category(tokens: &[&str]) -> Result<Option<CompetitionCategory>, SpecificationError> {
    tokens
        .iter()
        .filter_map(|token| gender(token))
        .try_fold(None, |old, value| {
            if old.as_ref().is_some_and(|old| old != &value) {
                return Err(SpecificationError::ConflictingSpecification);
            }
            Ok(Some(value))
        })
}

fn group_category<'a>(tokens: &[&'a str]) -> Result<Option<&'a str>, SpecificationError> {
    tokens
        .iter()
        .copied()
        .filter(|token| is_group(token))
        .try_fold(None, |old: Option<&str>, value| {
            if old.is_some_and(|old| !old.eq_ignore_ascii_case(value)) {
                return Err(SpecificationError::ConflictingSpecification);
            }
            Ok(Some(value))
        })
}

fn gender(token: &str) -> Option<CompetitionCategory> {
    if ["boys", "men", "mens", "male"]
        .iter()
        .any(|word| token.eq_ignore_ascii_case(word))
    {
        Some(CompetitionCategory::Boys)
    } else if ["girls", "women", "womens", "female"]
        .iter()
        .any(|word| token.eq_ignore_ascii_case(word))
    {
        Some(CompetitionCategory::Girls)
    } else if ["mixed", "coed"]
        .iter()
        .any(|word| token.eq_ignore_ascii_case(word))
    {
        Some(CompetitionCategory::Mixed)
    } else {
        None
    }
}

fn is_group(token: &str) -> bool {
    GROUPS.iter().any(|group| token.eq_ignore_ascii_case(group))
        || token
            .get(..1)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("u"))
            && token
                .get(1..)
                .is_some_and(|age| !age.is_empty() && age.bytes().all(|ch| ch.is_ascii_digit()))
}
