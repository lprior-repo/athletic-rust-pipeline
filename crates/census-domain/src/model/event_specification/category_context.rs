use super::*;
use crate::model::Gender;

#[cfg(test)]
mod tests;

impl EventSpecification {
    pub(in crate::model) fn category_in_context(
        &self,
        gender: Gender,
        division: Option<&str>,
    ) -> Result<Option<CompetitionCategory>, SpecificationError> {
        let gender = match gender {
            Gender::Boys => Some(CompetitionCategory::Boys),
            Gender::Girls => Some(CompetitionCategory::Girls),
            _ => None,
        };
        check_gender(&self.category, &gender)?;
        let category = self.category.clone().or(gender);
        let category = division_category(category, division)?;
        if let Some(category) = &category {
            category.validate()?;
        }
        Ok(category)
    }
}

fn check_gender(
    published: &Option<CompetitionCategory>,
    gender: &Option<CompetitionCategory>,
) -> Result<(), SpecificationError> {
    let claimed = match published {
        Some(CompetitionCategory::Published(label)) => published_category_gender(label),
        other => other.clone(),
    };
    if matches!(
        claimed.as_ref(),
        Some(CompetitionCategory::Boys | CompetitionCategory::Girls)
    ) && gender.is_some()
        && claimed.as_ref() != gender.as_ref()
    {
        return Err(SpecificationError::ConflictingSpecification);
    }
    Ok(())
}

fn division_category(
    category: Option<CompetitionCategory>,
    division: Option<&str>,
) -> Result<Option<CompetitionCategory>, SpecificationError> {
    match division {
        Some(division) => {
            let category = published_category_label(category, division);
            category.validate()?;
            Ok(Some(category))
        }
        None => Ok(category),
    }
}
