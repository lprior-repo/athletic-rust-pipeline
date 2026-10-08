use super::super::*;
use census_domain::model::{
    CourseIdentity, CourseMeasurement, CrossCountryContext, DistanceUnit,
    PublishedCourseConditions, PublishedDistance, SourceEventLabel,
};

pub(super) struct Course {
    pub(super) mark: String,
    pub(super) id: String,
    pub(super) conditions: String,
}

impl Course {
    pub(super) fn context(&self) -> String {
        format!("comparison same_course, comparison course {:?}@\"1\", distance 5000.000 m, course {:?}@\"1\", measurement short, conditions Some({:?})",
            self.id, self.id, self.conditions)
    }

    pub(super) fn summary(&self) -> String {
        format!("XC {} [xc, na, unknown, {}]", self.mark, self.context())
    }
}

pub(super) fn five_k_performance(fixture: &Fixture, sport: Sport) -> TestResult<(EventId, String)> {
    let meet_id = meet(
        &fixture.store,
        UsJurisdiction::Wisconsin,
        &format!("Published 5000m {}", sport.stable_key()),
        "2026-09-15",
        CompetitionLevel::Invitational,
        sport,
    )?;
    let event_id = if sport == Sport::CrossCountry {
        qualified_event(
            fixture,
            &meet_id,
            EventKind::Track5000m,
            "fixture-five-k",
            CourseMeasurement::PublishedMeasured,
            "dry",
        )?
    } else {
        event(&fixture.store, &meet_id, EventKind::Track5000m)?
    };
    let mark = Mark::TimeSeconds(ExactSeconds::parse("1800.00")?);
    let displayed = "30:00.00".to_owned();
    performance(
        &fixture.store,
        &PerformanceRow {
            athlete: &fixture.julian,
            school: &fixture.wi_school,
            meet: &meet_id,
            event: &event_id,
            date: "2026-09-15",
        },
        mark,
        "wiaa_results",
        "https://wiaa.test/5000m/results",
    )?;
    Ok((event_id, displayed))
}

fn qualified_event(
    fixture: &Fixture,
    meet: &MeetId,
    kind: EventKind,
    course: &str,
    measurement: CourseMeasurement,
    conditions: &str,
) -> TestResult<EventId> {
    let specification = EventSpecification {
        cross_country: Some(CrossCountryContext {
            distance: PublishedDistance::new(DistanceUnit::Metres, 5_000_000)?,
            course: Some(CourseIdentity::new(course, "1")?),
            measurement,
            conditions: Some(PublishedCourseConditions::try_from(conditions.to_owned())?),
        }),
        ..EventSpecification::default()
    };
    let label = if kind == EventKind::Track5000m {
        "5000m"
    } else {
        "Boys Cross Country"
    };
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet,
            kind,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        specification,
    )?;
    let source = SourceRef::new(
        "wiaa_results",
        Some(format!("https://fixtures.test/xc/{course}/results")),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    let mut capture = Evidence::parsed(source, "2026-09-20T00:00:00Z");
    capture.note = Some(format!("Synthetic published 5000m course {course}, version 1, {measurement:?}, conditions {conditions}"));
    event.evidence.push(capture);
    fixture.store.append(Table::Events, &event)?;
    Ok(event.id)
}

pub(super) fn five_k_context(policy: &str) -> String {
    let compared = if policy == "observed_fastest" {
        "any"
    } else {
        "\"fixture-five-k\"@\"1\""
    };
    format!("comparison {policy}, comparison course {compared}, distance 5000.000 m, course \"fixture-five-k\"@\"1\", measurement measured, conditions Some(\"dry\")")
}

fn course(fixture: &Fixture, index: usize, conditions: &str) -> TestResult<Course> {
    let id = format!("fixture-course-{index:03}");
    let meet_id = meet(
        &fixture.store,
        UsJurisdiction::Wisconsin,
        &format!("Synthetic published short XC course {index}"),
        "2026-09-15",
        CompetitionLevel::Invitational,
        Sport::CrossCountry,
    )?;
    let event = qualified_event(
        fixture,
        &meet_id,
        EventKind::CrossCountry,
        &id,
        CourseMeasurement::PublishedShort,
        conditions,
    )?;
    let seconds = i64::try_from(index)?
        .checked_add(1800)
        .ok_or("fixture seconds overflow")?;
    let nanos = seconds
        .checked_mul(1_000_000_000)
        .ok_or("fixture mark overflow")?;
    let mark = Mark::TimeSeconds(ExactSeconds::from_parts(nanos, 2)?);
    let displayed = format!(
        "{}:{:02}.00",
        seconds.div_euclid(60),
        seconds.rem_euclid(60)
    );
    performance(
        &fixture.store,
        &PerformanceRow {
            athlete: &fixture.julian,
            school: &fixture.wi_school,
            meet: &meet_id,
            event: &event,
            date: "2026-09-15",
        },
        mark,
        "wiaa_results",
        "https://wiaa.test/xc/results",
    )?;
    Ok(Course {
        mark: displayed,
        id,
        conditions: conditions.into(),
    })
}

pub(super) fn cross_country_contexts(fixture: &Fixture, count: usize) -> TestResult<Vec<Course>> {
    (0..count)
        .map(|index| course(fixture, index, "dry"))
        .collect()
}

pub(super) fn fitting_contexts(fixture: &Fixture) -> TestResult<Vec<Course>> {
    let mut used = 0usize;
    for index in 0usize..1000 {
        let candidate = Course {
            mark: "30:00.00".into(),
            id: format!("fixture-course-{index:03}"),
            conditions: "dry".into(),
        };
        let length = candidate.summary().encode_utf16().count();
        let separator = if index == 0 { 0 } else { 2 };
        let total = used
            .checked_add(separator)
            .and_then(|value| value.checked_add(length))
            .ok_or("fixture summary overflow")?;
        if total > 32_767 {
            let remainder = 32_767usize
                .checked_sub(used)
                .ok_or("fixture summary underflow")?;
            let last_index = index.checked_sub(1).ok_or("fixture index underflow")?;
            let mut courses = cross_country_contexts(fixture, last_index)?;
            courses.push(course(
                fixture,
                last_index,
                &format!("dry{}", "x".repeat(remainder)),
            )?);
            return Ok(courses);
        }
        used = total;
    }
    Err("fixture summary bound never reached".into())
}
