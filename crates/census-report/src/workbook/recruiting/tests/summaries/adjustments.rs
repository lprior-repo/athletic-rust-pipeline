use super::*;
use census_domain::model::Adjustment;

const ACTUAL: &str = "400m 48.55 [outdoor, na, unknown]";
const KNOWN: &str = "400m 48.00 [outdoor, na, unknown, conversion known from 48.55: Converted from 48.55 for Track Size. 🏃]";
const UNKNOWN: &str = "400m 47.90 [outdoor, na, unknown, conversion unknown]";
const NOTE: &str = "Converted from 48.55 for Track Size. 🏃";

fn seconds(value: &str) -> TestResult<Mark> {
    Ok(Mark::TimeSeconds(ExactSeconds::parse(value)?))
}

fn add_conversions(fixture: &Fixture) -> TestResult {
    let actual_mark = seconds("48.55")?;
    let actual = fixture
        .store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .find(|performance| {
            performance.athlete == fixture.julian && performance.mark == actual_mark
        })
        .ok_or("actual fixture performance missing")?;
    for (label, original, adjusted, note) in [
        ("known", Some("48.55"), "48.00", Some(NOTE)),
        ("unknown", None, "47.90", None),
    ] {
        let mut converted = actual.clone();
        converted.source_key = format!("{}:conversion-{label}", actual.source_key);
        converted.id = CanonicalPerformance::mint(
            &converted.athlete,
            &converted.meet,
            &converted.event,
            &converted.date,
            &converted.source_key,
        );
        converted.mark = seconds(adjusted)?;
        converted.adjustment = Some(Adjustment::track_size_conversion(
            original.map(seconds).transpose()?,
            converted.mark.clone(),
            note.map(str::to_owned),
            Some(SourceRef::new(
                "wiaa_results",
                Some("https://wiaa.test/results/invite".into()),
            )),
        ));
        fixture.store.append(Table::Performances, &converted)?;
    }
    Ok(())
}

fn forgeries(summary: &str) -> Vec<String> {
    [
        summary.replace(KNOWN, "400m 48.00 [outdoor, na, unknown]"),
        summary.replace(
            "conversion known from 48.55:",
            "conversion unknown from 48.55:",
        ),
        summary.replace(
            "conversion known from 48.55:",
            "conversion known from 49.00:",
        ),
        summary.replace(NOTE, "forged conversion note"),
        summary.replace(
            UNKNOWN,
            "400m 47.90 [outdoor, na, unknown, conversion known]",
        ),
        summary.replace(
            UNKNOWN,
            "400m 47.90 [outdoor, na, unknown, conversion unknown from 48.55]",
        ),
        summary.replace(
            ACTUAL,
            "400m 48.55 [outdoor, na, unknown, conversion known from 49.00]",
        ),
        String::new(),
    ]
    .into()
}

#[test]
fn frozen_readback_labels_actual_and_converted_marks_and_refuses_summary_forgeries() -> TestResult {
    let fixture = fixture()?;
    add_conversions(&fixture)?;
    let (published, dataset, options) = frozen_publication(&fixture)?;
    crate::workbook::verify::verify_frozen(&published, &dataset, &options)?;
    let mut book: Xlsx<_> = open_workbook(&published)?;
    let athletes = sheet(&mut book, "Athletes")?;
    let row = row_of(&athletes, fixture.julian.as_str())?;
    for (header, column_name) in [("Headline PR summary", "O"), ("400m (s)", "R")] {
        let value = text(&athletes, row, column_of(&athletes, header)?);
        for expected in [ACTUAL, KNOWN, UNKNOWN] {
            check!(value.split("; ").any(|entry| entry == expected), "{value}");
        }
        for (index, forged) in forgeries(&value).into_iter().enumerate() {
            check!(ne; forged, value);
            let target = fixture
                .dir
                .path()
                .join(format!("forged-{column_name}-{index}.xlsx"));
            forged_workbook(&published, &target, row, header, &forged)?;
            match crate::workbook::verify::verify_frozen(&target, &dataset, &options) {
                Err(crate::report::ReportError::SemanticMismatch { .. }) => (),
                Err(error) => return Err(format!("wrong summary refusal: {error}").into()),
                Ok(_) => return Err(format!("forged {header} accepted: {forged}").into()),
            }
        }
    }
    Ok(())
}
