use super::*;
use bytes::Bytes;

mod boundaries;
mod fixture;
mod hostile;
mod presentation_admission;
mod rebinding;
mod routing;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn encode(plan: &ResultsPlanV2) -> Result<Bytes, ResultsPlanError> {
    <ResultsPlanV2 as restate_sdk::serde::Serialize>::serialize(plan)
}

fn decode(bytes: &mut Bytes) -> Result<ResultsPlanV2, ResultsPlanError> {
    <ResultsPlanV2 as restate_sdk::serde::Deserialize>::deserialize(bytes)
}

fn limit(result: Result<ResultsPlanV2, ResultsPlanError>, expected: &'static str) -> TestResult {
    match result {
        Err(ResultsPlanError::Limit(actual)) => {
            check!(eq; actual, expected);
            Ok(())
        }
        other => Err(format!("expected plan limit {expected}, got {other:?}").into()),
    }
}

fn invalid(result: Result<ResultsPlanV2, ResultsPlanError>, expected: &'static str) -> TestResult {
    match result {
        Err(ResultsPlanError::Invalid(actual)) => {
            check!(eq; actual, expected);
            Ok(())
        }
        other => Err(format!("expected invalid plan {expected}, got {other:?}").into()),
    }
}

fn json_error(result: Result<ResultsPlanV2, ResultsPlanError>, expected: &str) -> TestResult {
    match result {
        Err(ResultsPlanError::Json(error)) => {
            check!(error.to_string().contains(expected));
            Ok(())
        }
        other => Err(format!("expected JSON refusal containing {expected}, got {other:?}").into()),
    }
}
