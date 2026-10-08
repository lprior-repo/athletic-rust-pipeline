use census_domain::model::{CourseMeasurement, DistanceUnit};

use super::{ComparisonPolicy, SharedSelection};

pub(crate) fn cross_country(pr: &SharedSelection) -> Option<String> {
    let context = pr.source.specification.cross_country.as_ref()?;
    let distance = context.distance.thousandths();
    let unit = match context.distance.unit {
        DistanceUnit::Metres => "m",
        DistanceUnit::Miles => "mi",
    };
    let course = context.course.as_ref();
    Some(format!(
        "comparison {}, comparison course {}, distance {}.{:03} {}, course {:?}@{:?}, measurement {}, conditions {:?}",
        policy(pr.key.comparison), comparison_course(pr),
        distance.div_euclid(1000), distance.rem_euclid(1000), unit,
        course.map_or("unknown", |course| course.id()),
        course.map_or("unknown", |course| course.version()),
        measurement(context.measurement),
        context.conditions.as_ref().map(|conditions| conditions.as_str()),
    ))
}

fn comparison_course(pr: &SharedSelection) -> String {
    match pr
        .key
        .specification
        .cross_country
        .as_ref()
        .and_then(|context| context.course.as_ref())
    {
        Some(course) => format!("{:?}@{:?}", course.id(), course.version()),
        None => "any".into(),
    }
}

fn policy(policy: ComparisonPolicy) -> &'static str {
    match policy {
        ComparisonPolicy::Standard => "standard",
        ComparisonPolicy::ObservedFastest => "observed_fastest",
        ComparisonPolicy::SameCourse => "same_course",
    }
}

fn measurement(measurement: CourseMeasurement) -> &'static str {
    match measurement {
        CourseMeasurement::PublishedMeasured => "measured",
        CourseMeasurement::PublishedShort => "short",
        CourseMeasurement::Unknown => "unknown",
    }
}
