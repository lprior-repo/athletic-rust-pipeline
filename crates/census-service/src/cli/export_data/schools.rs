use crate::cli::export_data::csv::write_csv;
use census_domain::model::{CanonicalSchool, SourceNamespace};
use std::collections::BTreeSet;

fn identity_fields(s: &CanonicalSchool) -> (String, String, usize) {
    let namespaces: BTreeSet<&SourceNamespace> =
        s.source_identities.iter().map(|i| &i.namespace).collect();
    let source_ns = namespaces
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(";");
    let evidence_src = s
        .evidence
        .iter()
        .map(|e| e.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(";");
    let ident_count = s.source_identities.len();
    (source_ns, evidence_src, ident_count)
}

fn build_school_row(s: &CanonicalSchool) -> anyhow::Result<Vec<String>> {
    let an_team = s
        .source_identities
        .iter()
        .find(|i| matches!(&i.namespace, SourceNamespace::LegacyAthleticNet { kind } | SourceNamespace::AthleticNet { kind } if kind == "team"))
        .map(|i| i.id.clone())
        .map_or(Default::default(), core::convert::identity);
    let ms_school = s
        .source_identities
        .iter()
        .find(|i| matches!(i.namespace, SourceNamespace::MilesplitSchool))
        .map(|i| i.id.clone())
        .map_or(Default::default(), core::convert::identity);
    let aliases = s.aliases.join(";");
    let (source_ns, evidence_src, ident_count) = identity_fields(s);

    let mut row = vec![
        s.id.as_str().to_string(),
        s.name.clone(),
        s.state
            .map(|j| j.code().to_owned())
            .map_or(Default::default(), core::convert::identity),
        s.city
            .clone()
            .map_or(Default::default(), core::convert::identity),
        s.association
            .clone()
            .map_or(Default::default(), core::convert::identity),
        s.classification
            .clone()
            .map_or(Default::default(), core::convert::identity),
        s.enrollment
            .map(|e| e.to_string())
            .map_or(Default::default(), core::convert::identity),
        if s.co_op {
            "true".to_string()
        } else {
            "false".to_string()
        },
        s.athletics_website
            .clone()
            .map_or(Default::default(), core::convert::identity),
        s.school_website
            .clone()
            .map_or(Default::default(), core::convert::identity),
        an_team,
        ms_school,
        aliases,
        source_ns,
        evidence_src,
        ident_count.to_string(),
    ];
    row.extend(census_report::export::postal::postal_fields([s])?);
    row.extend(census_report::export::link::link_fields(s));
    Ok(row)
}

const SCHOOL_IDENTITY_HEADERS: [&str; 16] = [
    "school_id",
    "name",
    "state",
    "city",
    "association",
    "classification",
    "enrollment",
    "co_op",
    "athletics_website",
    "school_website",
    "athleticnet_team_id",
    "milesplit_school_id",
    "aliases",
    "source_namespaces",
    "evidence_sources",
    "identity_count",
];

pub fn write_canonical_schools(
    schools: &[CanonicalSchool],
    data: &std::path::Path,
) -> anyhow::Result<()> {
    let mut sorted: Vec<&CanonicalSchool> = schools.iter().collect();
    sorted.sort_by(|a, b| {
        a.state
            .map(|j| j.code())
            .map_or(Default::default(), core::convert::identity)
            .cmp(
                b.state
                    .map(|j| j.code())
                    .map_or(Default::default(), core::convert::identity),
            )
            .then_with(|| a.name.cmp(&b.name))
    });
    let rows = sorted
        .into_iter()
        .map(build_school_row)
        .collect::<anyhow::Result<Vec<_>>>()?;

    write_csv(
        &data.join("canonical-schools.csv"),
        SCHOOL_IDENTITY_HEADERS
            .into_iter()
            .chain(census_report::export::postal::POSTAL_CSV_HEADERS)
            .chain(census_report::export::link::LINK_CSV_HEADERS),
        &rows,
    )?;

    Ok(())
}
