use super::super::super::{bound, invalid, markup, RetainedHtmlCaptureV1};
use super::super::{json, urls, CheckedMileSplitProfileSubjectV1, ProviderTargetProofV1};
use census_domain::model::ProfileDate;
use census_store::StoreResult;
use serde::Deserialize;

pub(super) struct Context {
    pub(super) name: String,
    pub(super) day: ProfileDate,
    pub(super) proof: ProviderTargetProofV1,
    pub(super) school_proof: ProviderTargetProofV1,
}

pub(super) fn read(
    capture: &RetainedHtmlCaptureV1,
    html: &str,
    profile: &CheckedMileSplitProfileSubjectV1,
) -> StoreResult<Context> {
    let school_proof = school(capture, html, profile)?;
    let selected = markup::elements(html, b"script", None)?
        .into_iter()
        .try_fold(None, |selected, script| -> StoreResult<_> {
            if !script
                .attribute("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("application/ld+json"))
            {
                return Ok(selected);
            }
            let event: Event = json::script(script.html)?;
            if event.graph.is_some() {
                return Err(invalid("linked competition JSON-LD graph unsupported"));
            }
            let kind = event
                .kind
                .as_deref()
                .ok_or_else(|| invalid("linked competition schema type absent"))?;
            if !matches!(
                kind,
                "SportsEvent" | "https://schema.org/SportsEvent" | "http://schema.org/SportsEvent"
            ) {
                return Ok(selected);
            }
            if selected.is_some()
                || event.url.as_deref() != Some(capture.requested_url())
                || event
                    .id
                    .as_ref()
                    .is_some_and(|id| id != capture.requested_url())
                || event.sport.as_deref() != Some("Outdoor Track & Field")
                || event
                    .location
                    .as_ref()
                    .and_then(|location| location.0.address.as_ref())
                    .and_then(|address| address.0.region.as_deref())
                    != Some("MI")
            {
                return Err(invalid(
                    "linked competition metadata owner or scope differs",
                ));
            }
            let name = event
                .name
                .ok_or_else(|| invalid("linked competition name absent"))?;
            bound(name.len(), 256, "linked competition name bytes")?;
            if name.is_empty() {
                return Err(invalid("linked competition name empty"));
            }
            let date = event
                .start
                .ok_or_else(|| invalid("linked competition date absent"))?;
            if event.end.as_deref() != Some(date.as_str()) {
                return Err(invalid("linked competition day is not exact"));
            }
            let day = ProfileDate::parse(&date)
                .map_err(|_| invalid("linked competition day unsupported"))?;
            Ok(Some((
                name,
                day,
                ProviderTargetProofV1::new(capture, script.locator()),
            )))
        })?;
    let (name, day, proof) =
        selected.ok_or_else(|| invalid("linked competition metadata absent"))?;
    Ok(Context {
        name,
        day,
        proof,
        school_proof,
    })
}

fn school(
    capture: &RetainedHtmlCaptureV1,
    html: &str,
    profile: &CheckedMileSplitProfileSubjectV1,
) -> StoreResult<ProviderTargetProofV1> {
    let headings = markup::elements(html, b"h2", None)?
        .into_iter()
        .map(|node| Ok((markup::text(node.html)?, node)))
        .collect::<StoreResult<Vec<_>>>()?;
    let selected = markup::single(
        headings
            .into_iter()
            .filter(|(text, _)| text.ends_with(" Results"))
            .map(|(_, node)| node)
            .collect(),
    )?;
    let link = markup::single(markup::elements(selected.html, b"a", None)?)?;
    let school = urls::school(
        link.attribute("href")
            .ok_or_else(|| invalid("linked competition school owner absent"))?,
    )?;
    if school.id != profile.school.id
        || markup::text(link.html)? != profile.school_name
        || markup::text(selected.html)? != format!("{} Results", profile.school_name)
    {
        return Err(invalid(
            "linked competition school differs from exact profile association",
        ));
    }
    Ok(ProviderTargetProofV1::new(capture, selected.locator()))
}

#[derive(Deserialize)]
struct Event {
    #[serde(rename = "@type")]
    kind: Option<String>,
    #[serde(rename = "@graph")]
    graph: Option<serde::de::IgnoredAny>,
    url: Option<String>,
    #[serde(rename = "@id")]
    id: Option<String>,
    name: Option<String>,
    #[serde(rename = "startDate")]
    start: Option<String>,
    #[serde(rename = "endDate")]
    end: Option<String>,
    sport: Option<String>,
    location: Option<json::MapOnly<Location>>,
}

#[derive(Deserialize)]
struct Location {
    address: Option<json::MapOnly<Address>>,
}

#[derive(Deserialize)]
struct Address {
    #[serde(rename = "addressRegion")]
    region: Option<String>,
}
