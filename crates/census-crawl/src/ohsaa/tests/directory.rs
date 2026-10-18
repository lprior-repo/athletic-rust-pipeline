use super::{capture, AD_FETCHED, SPORTS_FETCHED};
use crate::ohsaa::{school_entities, DirectorPage, SearchResult, SportsPage};
use census_domain::model::{Gender, Sport};

pub(super) const OWNED_AND_FOREIGN: &str = r#"<!doctype html><html><body>
<div class="schoolHeader"><h2>DUBLIN COFFMAN (474)</h2></div>
<table><tr class="informationSportHeaderRow"><th>Sport</th><th>Head Boys Coach</th><th>Head Girls Coach</th></tr>
<tr><td>Cross Country</td><td><a class="fieldValue" href="mailto:owned@example.org">Owned Coach</a></td><td><span class="fieldValue">N/A</span></td></tr></table>
<div class="schoolHeader"><h2>FOREIGN SCHOOL (999)</h2></div>
<table><tr><td>Cross Country</td><td><a class="fieldValue" href="mailto:foreign@example.org">Foreign Coach</a></td><td><span class="fieldValue">Foreign Girls Coach</span></td></tr></table>
</body></html>"#;

fn owner() -> SearchResult {
    SearchResult {
        name: "DUBLIN COFFMAN".to_string(),
        city: "Dublin".to_string(),
        ohsaa_id: "474".to_string(),
    }
}

#[test]
fn projection_uses_only_checked_rows_before_owned_table_close() -> anyhow::Result<()> {
    let owner = owner();
    let page = SportsPage::parse(
        &owner,
        capture(owner.sports_url(), OWNED_AND_FOREIGN, SPORTS_FETCHED),
    )?;
    let extract = school_entities(&page, None).map_err(anyhow::Error::msg)?;
    let facts: Vec<_> = extract
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.sport, coach.gender))
        .collect();
    check!(eq; facts, vec![("Owned Coach", Some(Sport::CrossCountry), Gender::Boys)]);
    check!(eq; extract.school.source_identities[0].id, "474");
    check!(eq; extract.coaches[0].school, extract.school_id);
    check!(eq; extract.coaches[0].professional_email.as_deref(), Some("owned@example.org"));
    Ok(())
}

#[test]
fn checked_projection_refuses_pairing_a_foreign_director_page() -> anyhow::Result<()> {
    let owner = owner();
    let sports = SportsPage::parse(
        &owner,
        capture(owner.sports_url(), OWNED_AND_FOREIGN, SPORTS_FETCHED),
    )?;
    let foreign = SearchResult {
        name: "CENTERVILLE".to_string(),
        city: "Centerville".to_string(),
        ohsaa_id: "336".to_string(),
    };
    let ad = DirectorPage::parse(
        &foreign,
        capture(
            foreign.ad_url(),
            super::fixture_ad_centerville(),
            AD_FETCHED,
        ),
    )?;
    let error = school_entities(&sports, Some(&ad)).err();
    check!(eq; error, Some("sports and director pages belong to different school owners"));
    Ok(())
}
