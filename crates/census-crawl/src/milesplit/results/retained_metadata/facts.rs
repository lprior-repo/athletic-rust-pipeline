use super::{RetainedPerformanceDate, RetainedResultDivision};
use census_domain::model::ProfileDate;
use serde_json::Value;

pub(super) fn date(start: &str, end: Option<&str>) -> RetainedPerformanceDate {
    let Some(start) = crate::context::published_performance_date(start) else {
        return RetainedPerformanceDate::Unresolved;
    };
    if end.is_some_and(|end| crate::context::published_performance_date(end) != Some(start)) {
        return RetainedPerformanceDate::Unresolved;
    }
    match ProfileDate::parse(&start.format("%Y-%m-%d").to_string()) {
        Ok(date) => RetainedPerformanceDate::Exact(date),
        Err(_) => RetainedPerformanceDate::Unresolved,
    }
}

pub(super) fn division(provider: &Value) -> RetainedResultDivision {
    let Some(label) = provider.get("divisionName").and_then(Value::as_str) else {
        return RetainedResultDivision::Unknown;
    };
    match label {
        "High School" => RetainedResultDivision::HighSchool,
        "Middle School" => RetainedResultDivision::MiddleSchool,
        "Collegiate" | "College" => RetainedResultDivision::Collegiate,
        "Post Graduation" => RetainedResultDivision::PostGraduation,
        "Club" => RetainedResultDivision::Club,
        "Open" => RetainedResultDivision::Open,
        "Unattached" => RetainedResultDivision::Unattached,
        _ => RetainedResultDivision::Unknown,
    }
}

#[cfg(test)]
mod tests;
