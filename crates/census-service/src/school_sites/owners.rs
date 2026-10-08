use super::PlannedSite;
use census_domain::model::{normalize_name, CanonicalSchool, EvidenceMethod};
use census_store::{Store, StoreError, StoreResult, Table};
use std::io::Write;

pub(super) enum Owner {
    Missing,
    Matched(Box<CanonicalSchool>),
    Ambiguous,
}

pub(super) fn match_window(store: &Store, sites: &[PlannedSite]) -> StoreResult<Vec<Owner>> {
    if sites.len() > 64 {
        return Err(invariant("school-site ownership window exceeds 64 rows"));
    }
    let mut owners = Vec::new();
    owners
        .try_reserve(sites.len())
        .map_err(|error| invariant(&error.to_string()))?;
    owners.resize_with(sites.len(), || Owner::Missing);
    let mut bytes = 0usize;
    store
        .snapshot()
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            sites.iter().zip(&mut owners).try_for_each(|(site, owner)| {
                if matching(site, &school) {
                    admit(owner, &school, &mut bytes)
                } else {
                    Ok(())
                }
            })
        })?;
    Ok(owners)
}

fn matching(site: &PlannedSite, school: &CanonicalSchool) -> bool {
    if school.state != Some(site.state) || !discovered(school) {
        return false;
    }
    let name = normalize_name(&site.school);
    if name != school.normalized_name
        && !school
            .aliases
            .iter()
            .any(|alias| normalize_name(alias) == name)
    {
        return false;
    }
    let Some(published) = school.school_website.as_deref().and_then(parsed) else {
        return false;
    };
    parsed(&site.website).is_some_and(|queued| queued == published)
}

fn discovered(school: &CanonicalSchool) -> bool {
    school
        .source_identities
        .iter()
        .any(|source| source.url.as_deref().and_then(parsed).is_some())
        && school.evidence.iter().any(|evidence| {
            matches!(
                evidence.method,
                EvidenceMethod::Fetched | EvidenceMethod::Parsed
            ) && evidence.source.url.as_deref().and_then(parsed).is_some()
        })
}

fn parsed(raw: &str) -> Option<url::Url> {
    let url = url::Url::parse(raw).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some()
        && url.fragment().is_none())
    .then_some(url)
}

fn admit(owner: &mut Owner, school: &CanonicalSchool, bytes: &mut usize) -> StoreResult<()> {
    match owner {
        Owner::Missing => {
            let length = serialized_size(school)?;
            let total = bytes
                .checked_add(length)
                .ok_or_else(|| invariant("school-site ownership byte counter overflow"))?;
            if total > 8 * 1024 * 1024 {
                return Err(invariant("school-site ownership window exceeds 8 MiB"));
            }
            *bytes = total;
            *owner = Owner::Matched(Box::new(school.clone()));
        }
        Owner::Matched(retained) if retained.id != school.id => {
            *bytes = bytes
                .checked_sub(serialized_size(retained)?)
                .ok_or_else(|| invariant("school-site ownership byte counter reversed"))?;
            *owner = Owner::Ambiguous;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn serialized_size<T: serde::Serialize>(row: &T) -> StoreResult<usize> {
    let mut size = Size(0);
    serde_json::to_writer(&mut size, row)
        .map_err(|error| invariant(&format!("measuring bounded school-site record: {error}")))?;
    Ok(size.0)
}

struct Size(usize);

impl Write for Size {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let length = self
            .0
            .checked_add(bytes.len())
            .filter(|length| *length <= 8 * 1024 * 1024)
            .ok_or_else(|| std::io::Error::other("school-site owner exceeds 8 MiB"))?;
        self.0 = length;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn invariant(detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: detail.to_owned(),
    }
}
