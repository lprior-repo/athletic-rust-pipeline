use super::super::{bound, invalid};
use census_domain::model::{SourceIdentity, SourceNamespace};
use census_store::StoreResult;

fn parsed(raw: &str) -> StoreResult<url::Url> {
    bound(raw.len(), 2048, "provider target URL bytes")?;
    let url = url::Url::parse(raw).map_err(|_| invalid("provider target URL invalid"))?;
    if raw.bytes().any(|byte| byte.is_ascii_control())
        || url.scheme() != "https"
        || url.host_str() != Some("mi.milesplit.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid("provider target URL scope unsupported"));
    }
    Ok(url)
}

pub(super) fn numeric(value: &str) -> StoreResult<&str> {
    if value.is_empty()
        || value.starts_with('0')
        || value.len() > 20
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || value.parse::<u64>().is_err()
    {
        return Err(invalid("provider target numeric identity invalid"));
    }
    Ok(value)
}

fn token(value: &str) -> StoreResult<&str> {
    numeric(value.split_once('-').map_or(value, |(id, _)| id))
}

pub(super) fn person(raw: &str) -> StoreResult<SourceIdentity> {
    identity(raw, "athletes", SourceNamespace::MilesplitAthlete)
}

pub(super) fn school(raw: &str) -> StoreResult<SourceIdentity> {
    identity(raw, "teams", SourceNamespace::MilesplitSchool)
}

fn identity(raw: &str, kind: &str, namespace: SourceNamespace) -> StoreResult<SourceIdentity> {
    let url = parsed(raw)?;
    let mut segments = url
        .path_segments()
        .ok_or_else(|| invalid("provider target path absent"))?;
    if segments.next() != Some(kind) {
        return Err(invalid("provider target path kind differs"));
    }
    let id = token(
        segments
            .next()
            .ok_or_else(|| invalid("provider target identity absent"))?,
    )?;
    if segments.next().is_some() {
        return Err(invalid("provider target path has unsupported suffix"));
    }
    Ok(SourceIdentity::new(namespace, id).with_url(raw.to_owned()))
}

pub(super) fn team_result(raw: &str) -> StoreResult<(String, String)> {
    let url = parsed(raw)?;
    let mut segments = url
        .path_segments()
        .ok_or_else(|| invalid("linked result path absent"))?;
    if segments.next() != Some("meets") {
        return Err(invalid("linked result meet absent"));
    }
    let meet = token(
        segments
            .next()
            .ok_or_else(|| invalid("linked result meet identity absent"))?,
    )?;
    if segments.next() != Some("teams") {
        return Err(invalid("linked result team absent"));
    }
    let team = numeric(
        segments
            .next()
            .ok_or_else(|| invalid("linked result team identity absent"))?,
    )?;
    if segments.next().is_some() {
        return Err(invalid("linked team result path suffix unsupported"));
    }
    Ok((meet.into(), team.into()))
}

pub(super) fn result_document(raw: &str, expected_meet: &str) -> StoreResult<String> {
    let url = parsed(raw)?;
    let mut segments = url
        .path_segments()
        .ok_or_else(|| invalid("linked result document path absent"))?;
    if segments.next() != Some("meets")
        || token(
            segments
                .next()
                .ok_or_else(|| invalid("linked document meet absent"))?,
        )? != expected_meet
        || segments.next() != Some("results")
    {
        return Err(invalid("linked result document owner differs"));
    }
    let document = numeric(
        segments
            .next()
            .ok_or_else(|| invalid("linked result document ID absent"))?,
    )?;
    if segments.next() != Some("auto") || segments.next().is_some() {
        return Err(invalid("linked result document representation unsupported"));
    }
    Ok(document.into())
}
