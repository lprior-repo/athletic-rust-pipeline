use super::{
    MhsaaClassificationCategoryV1 as Category, MhsaaClassificationTransitionV1 as Transition,
    MhsaaOriginalSpanV1 as Span, MhsaaTransitionBasisV1 as Basis,
};
use census_store::StoreResult;

const PICKFORD: &str = "Pickford’s boys track &amp; field team won the last three Upper Peninsula Division 2 titles and will move to Division 3 for 2027 after competing in Division 2 again this spring";
const NEWBERRY: &str = "Newberry – winners of the last two UP Division 3 titles – will move into Division 2 in 2027 after competing in Division 3 this season.";
const GIRLS: &str = "The Pickford and Newberry girls teams, both coming off Finals championships last spring as well, will make the same switches for 2027.";

pub(super) fn read<'a>(paragraph: &Span<'a>) -> StoreResult<[Transition<'a>; 4]> {
    let pickford = paragraph.one(PICKFORD)?;
    let newberry = paragraph.one(NEWBERRY)?;
    let girls = paragraph.one(GIRLS)?;
    Ok([
        pickford_boys(paragraph, &pickford)?,
        newberry_boys(paragraph, &pickford, &newberry)?,
        girls_transition(paragraph, &pickford, &girls, "Pickford", true)?,
        girls_transition(paragraph, &pickford, &girls, "Newberry", false)?,
    ])
}

fn pickford_boys<'a>(paragraph: &Span<'a>, clause: &Span<'a>) -> StoreResult<Transition<'a>> {
    Ok(Transition {
        school: clause.one("Pickford")?,
        category: Category::Boys,
        category_literal: clause.one("boys")?,
        sport_literal: clause.one("track &amp; field")?,
        spring_literal: paragraph.one("spring sports")?,
        peninsula_context: clause.one("Upper Peninsula")?,
        previous_division: clause
            .one("Upper Peninsula Division 2")?
            .one("Division 2")?,
        announced_division: clause.one("move to Division 3")?.one("Division 3")?,
        season_year: clause.one("2027")?,
        statement: clause.clone(),
        basis: Basis::DirectBoysStatement,
    })
}

fn newberry_boys<'a>(
    paragraph: &Span<'a>,
    pickford: &Span<'a>,
    clause: &Span<'a>,
) -> StoreResult<Transition<'a>> {
    Ok(Transition {
        school: clause.one("Newberry")?,
        category: Category::Boys,
        category_literal: pickford.one("boys")?,
        sport_literal: pickford.one("track &amp; field")?,
        spring_literal: paragraph.one("spring sports")?,
        peninsula_context: clause.one("UP")?,
        previous_division: clause.one("UP Division 3")?.one("Division 3")?,
        announced_division: clause.one("move into Division 2")?.one("Division 2")?,
        season_year: clause.one("2027")?,
        statement: clause.clone(),
        basis: Basis::CoordinatedBoysStatement,
    })
}

fn girls_transition<'a>(
    paragraph: &Span<'a>,
    pickford: &Span<'a>,
    girls: &Span<'a>,
    school: &str,
    is_pickford: bool,
) -> StoreResult<Transition<'a>> {
    let boys = if is_pickford {
        pickford.clone()
    } else {
        paragraph.one(NEWBERRY)?
    };
    let previous = if is_pickford {
        "Upper Peninsula Division 2"
    } else {
        "UP Division 3"
    };
    let announced = if is_pickford {
        "move to Division 3"
    } else {
        "move into Division 2"
    };
    Ok(Transition {
        school: girls.one(school)?,
        category: Category::Girls,
        category_literal: girls.one("girls")?,
        sport_literal: pickford.one("track &amp; field")?,
        spring_literal: paragraph.one("spring sports")?,
        peninsula_context: boys.one(if is_pickford { "Upper Peninsula" } else { "UP" })?,
        previous_division: boys.one(previous)?.one(if is_pickford {
            "Division 2"
        } else {
            "Division 3"
        })?,
        announced_division: boys.one(announced)?.one(if is_pickford {
            "Division 3"
        } else {
            "Division 2"
        })?,
        season_year: girls.one("2027")?,
        statement: girls.clone(),
        basis: Basis::GirlsSameSwitchesStatement,
    })
}
