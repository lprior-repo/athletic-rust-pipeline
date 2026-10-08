use census_domain::model::{CanonicalSchool, Evidence, SourceIdentity, SourceNamespace};

pub const LINK_HEADERS: [&str; 8] = [
    "Link School ID",
    "Link Owner Namespace",
    "Link Owner ID",
    "Link Owner URL",
    "Link Source",
    "Link Source URL",
    "Link Observed Date",
    "Link Note",
];

pub const LINK_CSV_HEADERS: [&str; 8] = [
    "link_school_id",
    "link_owner_namespace",
    "link_owner_id",
    "link_owner_url",
    "link_source",
    "link_source_url",
    "link_observed_date",
    "link_note",
];

pub fn link_fields(school: &CanonicalSchool) -> [String; 8] {
    let Some(identity) = association_identity(school) else {
        return Default::default();
    };
    let evidence = association_evidence(school, identity);
    [
        school.id.as_str().to_string(),
        identity.namespace.to_string(),
        identity.id.clone(),
        optional_text(identity.url.as_deref()),
        optional_text(evidence.map(|evidence| evidence.source.id.as_str())),
        optional_text(evidence.and_then(|evidence| evidence.source.url.as_deref())),
        optional_text(evidence.map(|evidence| evidence.observed_on.as_str())),
        optional_text(evidence.and_then(|evidence| evidence.note.as_deref())),
    ]
}

fn optional_text(value: Option<&str>) -> String {
    value.map_or("", core::convert::identity).to_string()
}

fn association_identity(school: &CanonicalSchool) -> Option<&SourceIdentity> {
    school
        .source_identities
        .iter()
        .filter(|identity| {
            matches!(
                identity.namespace,
                SourceNamespace::AssociationSchool { .. }
            )
        })
        .min_by(|left, right| {
            left.namespace
                .cmp(&right.namespace)
                .then_with(|| left.id.cmp(&right.id))
        })
}

fn association_evidence<'a>(
    school: &'a CanonicalSchool,
    identity: &SourceIdentity,
) -> Option<&'a Evidence> {
    let by_url = identity.url.as_deref().and_then(|url| {
        school
            .evidence
            .iter()
            .find(|evidence| evidence.source.url.as_deref() == Some(url))
    });
    by_url.or_else(|| {
        let SourceNamespace::AssociationSchool { association } = &identity.namespace else {
            return None;
        };
        school
            .evidence
            .iter()
            .find(|evidence| evidence.source.id == *association)
    })
}

#[cfg(test)]
pub(crate) mod tests;
