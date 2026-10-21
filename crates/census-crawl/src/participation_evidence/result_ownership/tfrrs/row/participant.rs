use super::{cells, invalid, types, Result};
use crate::participation_evidence::result_ownership::tfrrs::native_links;
use crate::tfrrs::YearToken;
use census_domain::model::Gender;
use url::Url;

pub(super) fn read(
    body: &str,
    cells: &[cells::Cell],
    base: &Url,
    gender: Gender,
) -> Result<types::Participant> {
    let athlete = super::named(body, cells::get(cells, "Athlete")?, base)?;
    let (athlete_id, athlete_team) = native_links::athlete(&athlete.url)?;
    let team = super::named(body, cells::get(cells, "Team")?, base)?;
    let team_slug = native_links::team(&team.url, gender)?;
    if athlete_team != team_slug {
        return Err(invalid("TFRRS athlete and row team native paths differ"));
    }
    Ok(types::Participant {
        athlete_id,
        athlete_path: athlete.url.path().into(),
        athlete_name: athlete.text,
        athlete_span: athlete.span,
        team_path: team.url.path().into(),
        team_name: team.text,
        team_span: team.span,
    })
}

pub(super) fn school_year(body: &str, cell: &cells::Cell) -> Result<types::PublishedYear> {
    let label = super::cell_text(body, cell)?;
    let grade = YearToken::parse(&label)
        .and_then(YearToken::grade)
        .ok_or_else(|| invalid("unsupported TFRRS row school grade"))?;
    Ok(types::PublishedYear {
        label,
        grade,
        span: cell.span.clone(),
    })
}
