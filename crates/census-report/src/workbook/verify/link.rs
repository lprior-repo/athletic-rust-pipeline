use census_domain::model::{CanonicalSchool, Evidence, SourceIdentity, SourceNamespace};

pub(in crate::workbook) fn fields(school: &CanonicalSchool) -> [String; 8] {
    let mut identities: Vec<&SourceIdentity> = school
        .source_identities
        .iter()
        .filter(|identity| {
            matches!(
                identity.namespace,
                SourceNamespace::AssociationSchool { .. }
            )
        })
        .collect();
    identities.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.id.cmp(&right.id))
    });
    let Some(identity) = identities.first() else {
        return Default::default();
    };
    let evidence = evidence_for(school, identity);
    [
        school.id.as_str().to_string(),
        identity.namespace.to_string(),
        identity.id.clone(),
        identity
            .url
            .clone()
            .map_or(Default::default(), core::convert::identity),
        evidence
            .map(|evidence| evidence.source.id.clone())
            .map_or(Default::default(), core::convert::identity),
        evidence
            .and_then(|evidence| evidence.source.url.clone())
            .map_or(Default::default(), core::convert::identity),
        evidence
            .map(|evidence| evidence.observed_on.clone())
            .map_or(Default::default(), core::convert::identity),
        evidence
            .and_then(|evidence| evidence.note.clone())
            .map_or(Default::default(), core::convert::identity),
    ]
}

fn evidence_for<'a>(
    school: &'a CanonicalSchool,
    identity: &SourceIdentity,
) -> Option<&'a Evidence> {
    let by_url = school.evidence.iter().find(|evidence| {
        identity
            .url
            .as_deref()
            .is_some_and(|url| evidence.source.url.as_deref() == Some(url))
    });
    by_url.or_else(|| match &identity.namespace {
        SourceNamespace::AssociationSchool { association } => school
            .evidence
            .iter()
            .find(|evidence| evidence.source.id == *association),
        _ => None,
    })
}
