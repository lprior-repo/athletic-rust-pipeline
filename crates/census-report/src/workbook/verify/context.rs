use crate::bests::{ComparisonPolicy, SharedSelection};
use census_domain::model::{CourseIdentity, CourseMeasurement, DistanceUnit};

pub(super) fn cross_country(row: &SharedSelection) -> Option<String> {
    let source = row.source.specification.cross_country.as_ref()?;
    let projected = row.key.specification.cross_country.as_ref();
    let compared_course = projected.and_then(|value| value.course.as_ref());
    let units = match source.distance.unit {
        DistanceUnit::Metres => "m",
        DistanceUnit::Miles => "mi",
    };
    let distance = source.distance.thousandths();
    let fields = [
        format!("comparison {}", policy(row.key.comparison)),
        format!("comparison course {}", course(compared_course, "any")),
        format!(
            "distance {}.{:03} {units}",
            distance.div_euclid(1000),
            distance.rem_euclid(1000)
        ),
        format!(
            "course {}",
            course(source.course.as_ref(), "\"unknown\"@\"unknown\"")
        ),
        format!("measurement {}", measurement(source.measurement)),
        format!(
            "conditions {:?}",
            source.conditions.as_ref().map(|value| value.as_str())
        ),
    ];
    Some(fields.join(", "))
}

fn course(value: Option<&CourseIdentity>, absent: &str) -> String {
    match value {
        Some(value) => format!("{:?}@{:?}", value.id(), value.version()),
        None => absent.to_owned(),
    }
}

fn policy(value: ComparisonPolicy) -> &'static str {
    match value {
        ComparisonPolicy::Standard => "standard",
        ComparisonPolicy::ObservedFastest => "observed_fastest",
        ComparisonPolicy::SameCourse => "same_course",
    }
}

fn measurement(value: CourseMeasurement) -> &'static str {
    match value {
        CourseMeasurement::PublishedMeasured => "measured",
        CourseMeasurement::PublishedShort => "short",
        CourseMeasurement::Unknown => "unknown",
    }
}
