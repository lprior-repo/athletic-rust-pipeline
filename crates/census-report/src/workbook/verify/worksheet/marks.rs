use super::invalid;
use crate::report::ReportResult;
use crate::workbook::reported::Projection;
use crate::workbook::verify::{
    canonical::Value, current_school::AffiliationExpectations, expectations::Expectations, labels,
    prs, reported,
};
use census_domain::model::CanonicalAthlete;

const FIELDS: [(usize, &str); 25] = [
    (7, "Event"),
    (9, "Calculated PR"),
    (10, "Mark Value"),
    (11, "Unit"),
    (19, "Surface"),
    (20, "Wind Class"),
    (21, "Timing Class"),
    (25, "Event Context"),
    (13, "PR date"),
    (14, "Meet"),
    (3, "School"),
    (4, "State"),
    (16, "Result URL"),
    (22, "Performance ID"),
    (23, "Meet ID"),
    (24, "Source Key"),
    (26, "Published mark"),
    (27, "Conversion basis"),
    (28, "Conversion note"),
    (18, "PR conflict"),
    (12, "Wind"),
    (6, "Sport"),
    (8, "Season"),
    (15, "Place"),
    (17, "Source count"),
];

pub(super) fn values(
    expected: &Expectations<'_>,
    affiliations: &AffiliationExpectations<'_>,
    athlete: &CanonicalAthlete,
    projection: Option<&Projection<'_>>,
) -> ReportResult<[Value; 3]> {
    let accepted = expected.prs_of(athlete.id.as_str()).map(|pr| {
        let values = prs::values(pr, expected, Some(affiliations))?.collect::<Vec<_>>();
        let text = format(&values, 0, "verified_eligible_best")?;
        comparison(text, &pr.key)
    });
    let bests = projection.into_iter().flat_map(|projection| {
        projection.bests_for(&athlete.id).map(move |best| {
            reported::prs::best::values(projection, best, expected)
                .and_then(|values| source_format(&values))
                .and_then(|text| comparison(text, &best.group.key))
        })
    });
    let reviews = projection.into_iter().flat_map(|projection| {
        projection
            .reviews
            .iter()
            .filter(|review| review.performance.athlete == athlete.id)
            .map(move |review| {
                reported::prs::review::values(projection, review, expected)
                    .and_then(|values| source_format(&values))
                    .and_then(|text| {
                        serde_json::to_string(review.interpretation)
                            .map(|interpretation| {
                                format!("{text}; Reported Event Interpretation: {interpretation}")
                            })
                            .map_err(|_| invalid("encoding frozen unresolved interpretation"))
                    })
            })
    });
    Ok([join(accepted)?, join(bests)?, join(reviews)?])
}

fn comparison(
    text: String,
    key: &census_domain::model::performance_comparison::PrKey,
) -> ReportResult<String> {
    serde_json::to_string(key)
        .map(|key| format!("{text}; Comparison class: {key}"))
        .map_err(|_| invalid("encoding frozen complete comparison class"))
}

fn source_format(values: &[Value]) -> ReportResult<String> {
    let Some(Value::Text(status)) = values.get(1) else {
        return Err(invalid("frozen worksheet source best lacks a disposition"));
    };
    format(values, 2, status)
}

fn format(values: &[Value], offset: usize, status: &str) -> ReportResult<String> {
    let mut result = vec![status.to_owned()];
    for (column, label) in FIELDS {
        if labels::PR_HEADERS.get(column).copied() != Some(label) {
            return Err(invalid(
                "frozen best-mark label differs from its expected position",
            ));
        }
        let index = column
            .checked_add(offset)
            .ok_or_else(|| invalid("frozen best-mark position overflow"))?;
        let value = values
            .get(index)
            .ok_or_else(|| invalid("frozen best-mark expected field absent"))?;
        let text = match value {
            Value::Empty => continue,
            Value::Text(text) if text.is_empty() => continue,
            Value::Text(text) => text.clone(),
            Value::Number(value) => value.to_string(),
        };
        result.push(format!("{label}: {text}"));
    }
    Ok(result.join("; "))
}

fn join(values: impl Iterator<Item = ReportResult<String>>) -> ReportResult<Value> {
    let mut output = String::new();
    for value in values {
        let value = value?;
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&value);
        if output.encode_utf16().count() > 32_767 {
            return Err(invalid(
                "complete frozen best-mark group exceeds the Excel cell budget",
            ));
        }
    }
    Ok(Value::text(output))
}
