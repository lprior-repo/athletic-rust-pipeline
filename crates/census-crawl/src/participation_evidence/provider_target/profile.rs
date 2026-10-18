use super::super::{bound, invalid, markup, RetainedHtmlCaptureV1};
use super::{
    json, urls, CheckedMileSplitProfileSubjectV1, ProviderTargetProofV1, UnmappedPersonGenderV1,
};
use census_domain::model::GradYear;
use census_store::{StoreError, StoreResult};
use serde::Deserialize;

pub(super) fn read(
    capture: &RetainedHtmlCaptureV1,
) -> StoreResult<CheckedMileSplitProfileSubjectV1> {
    let person = urls::person(capture.requested_url())?;
    let html = capture.body()?;
    markup::canonical(&html, capture.requested_url())?;
    let (heading, name) = heading_name(&html)?;
    let (person_gender, schema_name) = person_metadata(capture, &html)?;
    if name != schema_name {
        return Err(invalid("profile displayed and Person subject names differ"));
    }
    let info = markup::single(markup::elements(&html, b"p", Some("athlete-info"))?)?;
    let (school, school_name, school_node) = school_parts(&info)?;
    if markup::text(school_node.html)? != school_name {
        return Err(invalid("profile school text is ambiguous"));
    }
    let (cohort, cohort_node) = cohort_of(&info)?;
    let (locality, locality_node) = locality_of(&info)?;
    let proofs = vec![
        ProviderTargetProofV1::new(capture, heading.locator()),
        relative(capture, info.span.start, &school_node)?,
        relative(capture, info.span.start, &cohort_node)?,
        relative(capture, info.span.start, &locality_node)?,
    ];
    Ok(CheckedMileSplitProfileSubjectV1 {
        person,
        name,
        school,
        school_name,
        cohort,
        locality,
        person_gender,
        proofs,
    })
}

fn heading_name(html: &str) -> StoreResult<(markup::Element<'_>, String)> {
    let heading = markup::single(
        markup::elements(html, b"h1", None)?
            .into_iter()
            .filter(|node| node.attribute("id") == Some("athleteName"))
            .collect(),
    )?;
    let name = bounded_text(&heading, 256)?;
    Ok((heading, name))
}

fn school_parts<'a>(
    info: &markup::Element<'a>,
) -> StoreResult<(
    census_domain::model::SourceIdentity,
    String,
    markup::Element<'a>,
)> {
    let school_node = markup::single(markup::elements(
        info.html,
        b"span",
        Some("current-school"),
    )?)?;
    let school_link = markup::single(markup::elements(school_node.html, b"a", None)?)?;
    let school = urls::school(
        school_link
            .attribute("href")
            .ok_or_else(|| invalid("profile school owner link absent"))?,
    )?;
    let school_name = bounded_text(&school_link, 256)?;
    Ok((school, school_name, school_node))
}

fn cohort_of<'a>(info: &markup::Element<'a>) -> StoreResult<(GradYear, markup::Element<'a>)> {
    let cohort_node = markup::single(markup::elements(info.html, b"span", Some("grad-year"))?)?;
    let cohort_text = bounded_text(&cohort_node, 32)?;
    let year = cohort_text
        .strip_prefix("Class of ")
        .ok_or_else(|| invalid("profile published cohort format unsupported"))?;
    if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("profile cohort is not explicit"));
    }
    let cohort = year
        .parse::<i16>()
        .ok()
        .and_then(GradYear::new)
        .ok_or_else(|| invalid("profile cohort unsupported"))?;
    Ok((cohort, cohort_node))
}

fn locality_of<'a>(info: &markup::Element<'a>) -> StoreResult<(String, markup::Element<'a>)> {
    let locality_node = markup::single(markup::elements(info.html, b"span", Some("city-state"))?)?;
    let locality = bounded_text(&locality_node, 256)?;
    if !locality
        .strip_suffix(", MI")
        .is_some_and(|city| !city.is_empty() && !city.contains(','))
    {
        return Err(invalid(
            "profile locality is outside supported jurisdiction",
        ));
    }
    Ok((locality, locality_node))
}

fn bounded_text(node: &markup::Element<'_>, maximum: usize) -> StoreResult<String> {
    let text = markup::text(node.html)?;
    bound(text.len(), maximum, "provider profile text bytes")?;
    if text.is_empty() {
        return Err(invalid("provider profile statement is empty"));
    }
    Ok(text)
}

fn relative(
    capture: &RetainedHtmlCaptureV1,
    base: usize,
    node: &markup::Element<'_>,
) -> StoreResult<ProviderTargetProofV1> {
    let start = base
        .checked_add(node.span.start)
        .ok_or(StoreError::CounterOverflow)?;
    let end = base
        .checked_add(node.span.end)
        .ok_or(StoreError::CounterOverflow)?;
    Ok(ProviderTargetProofV1::new(
        capture,
        format!("html_bytes:{start}:{end}"),
    ))
}

#[derive(Deserialize)]
struct Person {
    #[serde(rename = "@type")]
    kind: Option<String>,
    #[serde(rename = "@graph")]
    graph: Option<serde::de::IgnoredAny>,
    url: Option<String>,
    #[serde(rename = "@id")]
    id: Option<String>,
    name: Option<String>,
    gender: Option<String>,
}

fn person_metadata(
    capture: &RetainedHtmlCaptureV1,
    html: &str,
) -> StoreResult<(UnmappedPersonGenderV1, String)> {
    let scripts = markup::elements(html, b"script", None)?;
    let selected = scripts
        .into_iter()
        .try_fold(None, |selected, script| -> StoreResult<_> {
            if !script
                .attribute("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("application/ld+json"))
            {
                return Ok(selected);
            }
            let person: Person = json::script(script.html)?;
            if person.graph.is_some() {
                return Err(invalid("provider profile JSON-LD graph unsupported"));
            }
            let kind = person
                .kind
                .as_deref()
                .ok_or_else(|| invalid("provider profile schema type absent"))?;
            if !matches!(
                kind,
                "Person" | "https://schema.org/Person" | "http://schema.org/Person"
            ) {
                return Ok(selected);
            }
            if selected.is_some()
                || person.url.as_deref() != Some(capture.requested_url())
                || person
                    .id
                    .as_ref()
                    .is_some_and(|id| id != capture.requested_url())
            {
                return Err(invalid("provider profile Person owner is ambiguous"));
            }
            let name = person
                .name
                .ok_or_else(|| invalid("provider profile Person name absent"))?;
            bound(name.len(), 256, "provider Person name bytes")?;
            if let Some(raw) = &person.gender {
                bound(raw.len(), 128, "provider Person raw gender bytes")?;
            }
            Ok(Some((
                UnmappedPersonGenderV1 {
                    raw: person.gender,
                    proof: ProviderTargetProofV1::new(capture, script.locator()),
                },
                name,
            )))
        })?;
    selected.ok_or_else(|| invalid("provider profile Person declaration absent"))
}
