use super::super::{dataset::Dataset, prs};
use super::invalid;
use crate::export::current_school::SelectedSchools;
use crate::report::ReportResult;
use crate::workbook::cells::Cell;
use crate::workbook::reported::Projection;
use census_domain::model::CanonicalAthlete;

#[cfg(test)]
mod tests;

const FIELDS: [usize; 25] = [
    7, 9, 10, 11, 19, 20, 21, 25, 13, 14, 3, 4, 16, 22, 23, 24, 26, 27, 28, 18, 12, 6, 8, 15, 17,
];

pub(super) fn cells(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    reported: Option<&Projection<'_>>,
    schools: &SelectedSchools<'_>,
) -> ReportResult<[Cell; 3]> {
    let accepted = dataset.prs_of(athlete.id.as_str()).map(|pr| {
        prs::cells(pr, dataset.public_identities, Some(schools))
            .and_then(|cells| detail(&cells, 0, "verified_eligible_best"))
            .and_then(|text| comparison(text, &pr.key))
    });
    let bests = reported.into_iter().flat_map(|projection| {
        projection.bests_for(&athlete.id).map(move |best| {
            prs::reported::row(best, projection, true, true)
                .and_then(|cells| reported_detail(&cells))
                .and_then(|text| comparison(text, &best.group.key))
        })
    });
    let unresolved = reported.into_iter().flat_map(|projection| {
        projection
            .reviews
            .iter()
            .filter(|review| review.performance.athlete == athlete.id)
            .map(move |review| {
                prs::review::row(review, projection, true, true)
                    .and_then(|cells| reported_detail(&cells))
                    .and_then(|text| {
                        serde_json::to_string(review.interpretation)
                            .map(|interpretation| {
                                format!("{text}; Reported Event Interpretation: {interpretation}")
                            })
                            .map_err(|_| invalid("encoding worksheet unresolved interpretation"))
                    })
            })
    });
    Ok([combine(accepted)?, combine(bests)?, combine(unresolved)?])
}

fn comparison(
    text: String,
    key: &census_domain::model::performance_comparison::PrKey,
) -> ReportResult<String> {
    serde_json::to_string(key)
        .map(|key| format!("{text}; Comparison class: {key}"))
        .map_err(|_| invalid("encoding complete worksheet comparison class"))
}

fn reported_detail(cells: &[Cell]) -> ReportResult<String> {
    let Some(Cell::Text(status)) = cells.get(1) else {
        return Err(invalid(
            "reported worksheet mark lacks its verification status",
        ));
    };
    detail(cells, 2, status)
}

fn detail(cells: &[Cell], offset: usize, status: &str) -> ReportResult<String> {
    let mut parts = vec![status.to_owned()];
    for index in FIELDS {
        let label = prs::HEADERS
            .get(index)
            .ok_or_else(|| invalid("worksheet mark label is absent"))?;
        let position = index
            .checked_add(offset)
            .ok_or_else(|| invalid("worksheet mark position overflow"))?;
        let value = cells
            .get(position)
            .ok_or_else(|| invalid("worksheet mark field is absent"))?;
        let value = match value {
            Cell::Empty => continue,
            Cell::Text(text) if text.is_empty() => continue,
            Cell::Text(text) => text.clone(),
            Cell::Number(number) => number.to_string(),
        };
        parts.push(format!("{label}: {value}"));
    }
    Ok(parts.join("; "))
}

fn combine(rows: impl Iterator<Item = ReportResult<String>>) -> ReportResult<Cell> {
    let mut text = String::new();
    let mut units = 0usize;
    for row in rows {
        let row = row?;
        let separator = usize::from(!text.is_empty());
        units = units
            .checked_add(row.encode_utf16().count())
            .and_then(|units| units.checked_add(separator))
            .ok_or_else(|| invalid("worksheet best-mark text length overflow"))?;
        if units > 32_767 {
            return Err(invalid("complete worksheet best marks exceed the Excel cell budget; no marks were truncated"));
        }
        text.try_reserve(row.len().saturating_add(separator))
            .map_err(|_| invalid("reserving worksheet best-mark text"))?;
        if separator != 0 {
            text.push('\n');
        }
        text.push_str(&row);
    }
    if text.is_empty() {
        Ok(Cell::Empty)
    } else {
        Ok(Cell::text(text))
    }
}
