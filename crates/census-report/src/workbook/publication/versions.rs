use crate::report::ReportResult;

pub(super) fn validate_policy(artifact: u32, policy: u32) -> ReportResult<()> {
    match (artifact, policy) {
        (1..=5, 0..=20) | (6..=8, 21) => Ok(()),
        _ => Err(super::invariant(format!(
            "publication artifact revision {artifact} is incompatible with selection policy {policy}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::EventSemantics;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn current_artifact_requires_result_group_policy_and_historical_artifacts_refuse_it(
    ) -> TestResult {
        validate_policy(6, 21)?;
        validate_policy(7, 21)?;
        validate_policy(8, 21)?;
        for artifact in 1..=5 {
            check!(validate_policy(artifact, 21).is_err());
            for policy in 18..=20 {
                validate_policy(artifact, policy)?;
                check!(validate_policy(6, policy).is_err());
                check!(validate_policy(7, policy).is_err());
                check!(validate_policy(8, policy).is_err());
            }
        }
        for pair in [
            (0, 21),
            (9, 21),
            (8, 22),
            (7, 22),
            (6, 22),
            (5, 22),
            (u32::MAX, u32::MAX),
        ] {
            check!(validate_policy(pair.0, pair.1).is_err());
        }
        Ok(())
    }

    #[test]
    fn retained_structural_policy_metadata_does_not_broaden_semantic_decode() -> TestResult {
        for (artifact, policy) in [(1, 13), (3, 16), (4, 17), (5, 17)] {
            validate_policy(artifact, policy)?;
            check!(EventSemantics::from_revision(policy).is_none());
            check!(validate_policy(6, policy).is_err());
        }
        Ok(())
    }
}
