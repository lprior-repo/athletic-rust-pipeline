use crate::grouping::{add, Admission, GroupingError};
use census_domain::model::CanonicalPerformance;

const STAGE: &str = "best-mark supporting references";
const PER_CLASS_REFERENCE_ALLOWANCE: usize = 128;

pub(super) fn admit(performances: &[&CanonicalPerformance]) -> Result<(), GroupingError> {
    let mut admission = Admission::new(STAGE);
    performances
        .iter()
        .try_for_each(|performance| charge(&mut admission, performance.id.as_str().len()))
}

fn charge(admission: &mut Admission, id_bytes: usize) -> Result<(), GroupingError> {
    let one_class = add(
        STAGE,
        "reference allowance",
        id_bytes,
        PER_CLASS_REFERENCE_ALLOWANCE,
    )?;
    let both_classes = add(STAGE, "reference allowance", one_class, one_class)?;
    admission.charge(both_classes, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn supporting_reference_admission_accounts_for_two_classes_and_refuses_whole() -> TestResult {
        let mut admission = Admission::new(STAGE);
        let id_bytes = crate::grouping::STAGE_BYTES / 2 - PER_CLASS_REFERENCE_ALLOWANCE;
        charge(&mut admission, id_bytes)?;
        check!(matches!(
            charge(&mut admission, 1),
            Err(GroupingError::BudgetExceeded { .. })
        ));
        Ok(())
    }

    #[test]
    fn supporting_reference_admission_refuses_oversized_and_overflowing_ids() -> TestResult {
        let mut admission = Admission::new(STAGE);
        check!(matches!(
            charge(&mut admission, crate::grouping::STAGE_BYTES),
            Err(GroupingError::BudgetExceeded { .. })
        ));
        check!(matches!(
            charge(&mut admission, usize::MAX),
            Err(GroupingError::Overflow { .. })
        ));
        charge(&mut admission, 32)?;
        Ok(())
    }
}
