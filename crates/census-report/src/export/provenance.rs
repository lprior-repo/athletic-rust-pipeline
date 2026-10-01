use census_domain::model::{CanonicalCoach, CanonicalMeet, SourceIdentity, SourceNamespace};

pub fn coach_source(coach: &CanonicalCoach) -> (Option<&str>, Option<&str>) {
    let newest = coach
        .evidence
        .iter()
        .filter(|evidence| {
            evidence
                .source
                .url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
        })
        .max_by_key(|evidence| {
            (
                evidence.observed_on.as_str(),
                evidence.source.url.as_deref(),
            )
        });
    if let Some(evidence) = newest {
        return (
            evidence.source.url.as_deref(),
            Some(evidence.observed_on.as_str()).filter(|date| !date.is_empty()),
        );
    }
    (
        coach
            .source_identities
            .iter()
            .filter_map(|identity| identity.url.as_deref())
            .filter(|url| !url.trim().is_empty())
            .max(),
        None,
    )
}

pub fn athletic_net_meet_identity(meet: &CanonicalMeet) -> Option<&SourceIdentity> {
    meet.source_identities.iter().find(|identity| {
        matches!(
            &identity.namespace,
            SourceNamespace::LegacyAthleticNet { kind } | SourceNamespace::AthleticNet { kind }
                if kind == "meet"
        )
    })
}
