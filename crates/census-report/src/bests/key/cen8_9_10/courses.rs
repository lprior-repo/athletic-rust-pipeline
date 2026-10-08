use super::*;
#[test]
fn cen8_distance_fastest_and_same_course_cross_dates_are_separate_policies() -> TestResult {
    let mut data = dataset()?;
    for (name, course, version, time) in [
        ("A_early", "A", "1", "1000"),
        ("A_late", "A", "1", "990"),
        ("B", "B", "1", "980"),
        ("A_v2", "A", "2", "970"),
    ] {
        let date = match name {
            "A_early" => "2024-10-01",
            "A_v2" => "2023-10-01",
            _ => "2025-10-01",
        };
        let meet = meet(name, Sport::CrossCountry, date);
        let specification = xc(course, version, DistanceUnit::Metres, 5_000_000)?;
        let event =
            event_with_specification(&meet, "Boys Cross Country", Gender::Boys, specification)?;
        add(&mut data, meet, event, time)?;
    }
    let rows = selections(&data);
    let fastest = rows
        .iter()
        .find(|row| row.key.comparison == ComparisonPolicy::ObservedFastest)
        .ok_or("distance fastest")?;
    assert_eq!(
        fastest.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("970")?)
    );
    assert_eq!(fastest.population.marks, 4);
    assert_eq!(
        fastest
            .source
            .specification
            .cross_country
            .as_ref()
            .ok_or("source context")?
            .course,
        Some(CourseIdentity::new("A", "2")?)
    );
    assert_eq!(
        fastest
            .key
            .specification
            .cross_country
            .as_ref()
            .ok_or("projected context")?
            .course,
        None
    );
    assert_eq!(fastest.meet.date, "2023-10-01");
    assert_eq!(
        fastest.source.specification,
        EventSpecification {
            category: Some(CompetitionCategory::Boys),
            ..xc("A", "2", DistanceUnit::Metres, 5_000_000)?
        }
    );
    let course_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.key.comparison == ComparisonPolicy::SameCourse)
        .collect();
    assert_eq!(course_rows.len(), 3);
    let course_a = CourseIdentity::new("A", "1")?;
    let a = course_rows
        .iter()
        .find(|row| {
            row.key
                .specification
                .cross_country
                .as_ref()
                .is_some_and(|context| context.course.as_ref() == Some(&course_a))
        })
        .ok_or("A version1 winner")?;
    assert_eq!(
        a.result.mark,
        Mark::TimeSeconds(ExactSeconds::parse("990")?)
    );
    assert_eq!(a.population.marks, 2);
    let directory = tempfile::tempdir()?;
    let (jsonl, _) = crate::bests::write(directory.path(), &rows, "co2027")?;
    let text = std::fs::read_to_string(jsonl)?;
    let records: Vec<serde_json::Value> = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let published = records
        .iter()
        .find(|row| {
            row.pointer("/key/comparison")
                .and_then(serde_json::Value::as_str)
                == Some("observed_fastest")
        })
        .ok_or("published fastest")?;
    assert_eq!(
        published.pointer("/specification/cross_country/course"),
        Some(&serde_json::json!(["A", "2"]))
    );
    assert_eq!(
        published.pointer("/key/specification/cross_country/course"),
        Some(&serde_json::Value::Null)
    );
    assert_eq!(
        published.get("date"),
        Some(&serde_json::json!("2023-10-01"))
    );
    Ok(())
}

#[test]
fn cen8_three_miles_short_unknown_and_conditions_cannot_enter_standard_five_km_slot() -> TestResult
{
    let mut data = dataset()?;
    for (name, unit, distance, measurement, course, conditions, time) in [
        (
            "5km",
            DistanceUnit::Metres,
            5_000_000,
            CourseMeasurement::PublishedMeasured,
            Some("A"),
            None,
            "1000",
        ),
        (
            "3mi",
            DistanceUnit::Miles,
            3000,
            CourseMeasurement::PublishedMeasured,
            Some("A"),
            None,
            "900",
        ),
        (
            "short",
            DistanceUnit::Metres,
            5_000_000,
            CourseMeasurement::PublishedShort,
            Some("A"),
            None,
            "800",
        ),
        (
            "unknown",
            DistanceUnit::Metres,
            5_000_000,
            CourseMeasurement::Unknown,
            Some("A"),
            None,
            "700",
        ),
        (
            "no_course",
            DistanceUnit::Metres,
            5_000_000,
            CourseMeasurement::PublishedMeasured,
            None,
            None,
            "750",
        ),
        (
            "wet",
            DistanceUnit::Metres,
            5_000_000,
            CourseMeasurement::PublishedMeasured,
            Some("A"),
            Some("wet"),
            "950",
        ),
    ] {
        let meet = meet(name, Sport::CrossCountry, "2025-10-01");
        let specification = EventSpecification {
            cross_country: Some(CrossCountryContext {
                distance: PublishedDistance::new(unit, distance)?,
                course: course
                    .map(|course| CourseIdentity::new(course, "1"))
                    .transpose()?,
                measurement,
                conditions: conditions
                    .map(|value| PublishedCourseConditions::try_from(value.to_string()))
                    .transpose()?,
            }),
            ..EventSpecification::default()
        };
        let event =
            event_with_specification(&meet, "Boys Cross Country", Gender::Boys, specification)?;
        add(&mut data, meet, event, time)?;
    }
    let unknown = meet("no_distance", Sport::CrossCountry, "2025-10-01");
    let unknown_event = event(&unknown, "Boys Cross Country")?;
    add(&mut data, unknown, unknown_event, "600")?;
    let rows = selections(&data);
    let times: Vec<_> = rows.iter().map(|row| &row.result.mark).collect();
    assert_eq!(
        rows.iter()
            .filter(|row| row.key.comparison == ComparisonPolicy::ObservedFastest)
            .count(),
        3
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.key.comparison == ComparisonPolicy::SameCourse)
            .count(),
        4
    );
    assert!(!times.contains(&&Mark::TimeSeconds(ExactSeconds::parse("700")?)));
    assert!(!times.contains(&&Mark::TimeSeconds(ExactSeconds::parse("600")?)));
    assert!(!times.contains(&&Mark::TimeSeconds(ExactSeconds::parse("750")?)));
    let short_mark = Mark::TimeSeconds(ExactSeconds::parse("800")?);
    assert_eq!(times.iter().filter(|mark| ***mark == short_mark).count(), 1);
    let wet_mark = Mark::TimeSeconds(ExactSeconds::parse("950")?);
    let wet = rows
        .iter()
        .find(|row| {
            row.key.comparison == ComparisonPolicy::ObservedFastest && row.result.mark == wet_mark
        })
        .ok_or("wet course winner")?;
    let context = wet
        .source
        .specification
        .cross_country
        .as_ref()
        .ok_or("wet source context")?;
    assert_eq!(context.course, Some(CourseIdentity::new("A", "1")?));
    assert_eq!(
        context.conditions,
        Some(PublishedCourseConditions::try_from("wet".to_string())?)
    );
    let projected = wet
        .key
        .specification
        .cross_country
        .as_ref()
        .ok_or("wet projected context")?;
    assert_eq!(projected.course, None);
    assert_eq!(projected.conditions, context.conditions);
    Ok(())
}
