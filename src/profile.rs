mod bio;
mod html;
mod merge;
mod parser;
pub(crate) use parser::parse_bio_value;
pub use parser::{merge_profiles, parse_bio, parse_profile_html, HtmlProfileEvidence, TreeHint};

pub(crate) fn parse_location(
    team: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<Option<crate::domain::facts::Location>> {
    use crate::domain::facts::{CityName, Location, RegionName};
    use anyhow::Context;
    let city = location_text(team.get("City"), "City")?
        .map(CityName::parse)
        .transpose()
        .context("invalid team city")?;
    let region = location_text(team.get("State"), "State")?
        .map(RegionName::parse)
        .transpose()
        .context("invalid team region")?;
    Ok(match (city, region) {
        (Some(city), Some(region)) => Some(Location::CityRegion { city, region }),
        (Some(city), None) => Some(Location::CityOnly(city)),
        (None, Some(region)) => Some(Location::RegionOnly(region)),
        (None, None) => None,
    })
}

fn location_text<'a>(
    value: Option<&'a serde_json::Value>,
    field: &str,
) -> anyhow::Result<Option<&'a str>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let text = value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("team {field} is not text"))?
        .trim();
    Ok((!text.is_empty()).then_some(text))
}
