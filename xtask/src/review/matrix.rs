#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

use crate::paths::repo_root;

mod applicability;
mod arms;
mod families;
mod output;

use applicability::{read_applicability, read_arbiter, read_jurisdictions};
use arms::{read_source_constant, Arms};
use families::{build_families, parse_strategies};
use output::{jurisdiction_map, write_or_print};

const APPLICABILITY_FILE: &str = "crates/census-crawl/src/applicability/table/data.rs";
const APPLICABILITY_TABLE_FILE: &str = "crates/census-crawl/src/applicability/table.rs";
const JURISDICTION_FILE: &str = "crates/census-domain/src/jurisdiction/table.rs";
const TEAMS_FILE: &str = "crates/census-service/src/restate_services/teams_arms.rs";
const MEETS_FILE: &str = "crates/census-service/src/restate_services/meets_arms.rs";
const RESULTS_FILE: &str = "crates/census-service/src/restate_services/results_arms.rs";
const STRATEGIES_FILE: &str = "crates/census-service/src/restate_services/plan/strategies.rs";
const CENSUS_SOURCE_FILE: &str = "crates/census-service/src/census/meets.rs";

pub(super) fn run(out: Option<&Path>) -> Result<()> {
    let root = repo_root();
    let jurisdictions = read_jurisdictions(&root)?;
    let applicability = read_applicability(&root, &jurisdictions)?;
    let arbiter = read_arbiter(&root, &jurisdictions)?;
    let source_constant = read_source_constant(&root)?;
    let arms = Arms::parse(&root, &source_constant)?;
    let strategies = parse_strategies(&root)?;

    let families = build_families(&applicability, &arbiter, &arms, &strategies);

    let registered = families.len();
    let armed = families
        .iter()
        .filter(|f| f["strategy"] != "engineering_gap")
        .count();
    let gap = families
        .iter()
        .filter(|f| f["strategy"] == "engineering_gap")
        .count();

    let jurisdiction_map = jurisdiction_map(&jurisdictions, &families);

    let registered_slugs: Vec<String> = families
        .iter()
        .filter_map(|f| f["slug"].as_str().map(|s| s.to_string()))
        .collect();
    let arms_not_registered: Vec<String> = arms
        .slugs()
        .into_iter()
        .filter(|s| !registered_slugs.contains(s))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let files = vec![
        APPLICABILITY_FILE,
        APPLICABILITY_TABLE_FILE,
        JURISDICTION_FILE,
        TEAMS_FILE,
        MEETS_FILE,
        RESULTS_FILE,
        STRATEGIES_FILE,
        CENSUS_SOURCE_FILE,
    ];

    let output = json!({
        "schema": "census-source-jurisdiction-matrix/v1",
        "extracted_from": files,
        "registered_families": registered,
        "armed_families": armed,
        "gap_families": gap,
        "jurisdiction_count": jurisdictions.len(),
        "families": families,
        "jurisdictions": jurisdiction_map,
        "arms_not_registered": arms_not_registered
    });

    let rendered =
        serde_json::to_string_pretty(&output).with_context(|| "serializing matrix JSON")? + "\n";

    write_or_print(out, &rendered)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::applicability::{parse_applicability_entries, parse_declared_length};
    use super::arms::{resolve_slug, Arms};
    use super::families::determine_strategy;
    use anyhow::{ensure, Result};
    use indexmap::IndexMap;

    #[test]
    fn slug_extraction_from_sample() -> Result<()> {
        let content = r#"Applicability {
    slug: "aia",
    jurisdictions: &[UsJurisdiction::Arizona],
}"#;
        let entries = parse_applicability_entries(content, &[])?;
        ensure!(entries.len() == 1);
        ensure!(entries[0]["slug"] == "aia");
        ensure!(entries[0]["applicable"][0] == "Arizona");
        Ok(())
    }

    #[test]
    fn census_scope_expansion_in_applicability() -> Result<()> {
        let content = r#"Applicability {
    slug: "milesplit",
    jurisdictions: &[UsJurisdiction::CENSUS_SCOPE],
}"#;
        let scope = vec!["Alabama".to_string(), "Wyoming".to_string()];
        let entries = parse_applicability_entries(content, &scope)?;
        ensure!(entries.len() == 1);
        ensure!(entries[0]["applicable"][0] == "Alabama");
        ensure!(entries[0]["applicable"][1] == "Wyoming");
        Ok(())
    }

    #[test]
    fn unsupported_jurisdiction_constant_rejected() {
        let content = r#"Applicability {
    slug: "mystery",
    jurisdictions: &UsJurisdiction::SOMEWHERE_ELSE,
}"#;
        let result = parse_applicability_entries(content, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn declared_length_mismatch_detected() -> Result<()> {
        let content = "pub(crate) const TABLE: [Applicability; 100] = [\n";
        let declared = parse_declared_length(content, "TABLE", "Applicability")?;
        ensure!(declared == 100);
        let parsed = parse_applicability_entries(content, &[])?;
        ensure!(parsed.is_empty());
        ensure!(parsed.len() != declared);
        Ok(())
    }

    #[test]
    fn unresolved_constant_error() {
        let result = resolve_slug("UNKNOWN_CONSTANT", &IndexMap::new(), "test.rs");
        assert!(result.is_err());
        let Err(error) = result else {
            return;
        };
        assert!(error.to_string().contains("UNKNOWN_CONSTANT"));
    }

    #[test]
    fn strategy_precedence_mapping() {
        let arms = Arms {
            teams: vec![("milesplit".to_string(), "MilesplitIndex".to_string())],
            meets: vec![("wiaa_results".to_string(), "WiaaResults".to_string())],
            results: vec![("athleticnet".to_string(), "AthleticnetMeets".to_string())],
        };

        assert_eq!(determine_strategy("milesplit", &arms), "teams");
        assert_eq!(determine_strategy("wiaa_results", &arms), "meets");
        assert_eq!(determine_strategy("athleticnet", &arms), "results");
        assert_eq!(determine_strategy("foo", &arms), "engineering_gap");
    }

    #[test]
    fn ref_ordering_deterministic() -> Result<()> {
        let content = r#"Applicability {
    slug: "z",
    jurisdictions: &[UsJurisdiction::Wyoming, UsJurisdiction::Alabama],
},
Applicability {
    slug: "a",
    jurisdictions: &[UsJurisdiction::Wyoming, UsJurisdiction::Alabama],
},"#;
        let entries = parse_applicability_entries(content, &[])?;
        ensure!(entries.len() == 2);
        ensure!(entries[0]["slug"] == "z");
        ensure!(entries[0]["applicable"][0] == "Wyoming");
        ensure!(entries[0]["applicable"][1] == "Alabama");
        ensure!(entries[1]["slug"] == "a");
        Ok(())
    }
}
