use super::recruiting::{athletes_event_key, pr_event_name};
use crate::export::ReportedProfilePrTable;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::workbook) struct GapLine {
    pub(in crate::workbook) event_key: &'static str,
    pub(in crate::workbook) line: String,
}

pub(in crate::workbook) fn display(event_key: &str) -> String {
    pr_event_name(event_key).to_owned()
}

pub(in crate::workbook) fn gap_lines(
    covered: &BTreeSet<&str>,
    tables: &[&ReportedProfilePrTable],
) -> Vec<GapLine> {
    let mut seen = BTreeSet::new();
    let mut gaps = Vec::new();
    for table in tables {
        for row in &table.rows {
            let Some(key) = athletes_event_key(row.event()) else {
                continue;
            };
            if covered.contains(key) || !seen.insert(key) {
                continue;
            }
            gaps.push(GapLine {
                event_key: key,
                line: format!(
                    "{} {} [source-declared profile {}/{} {}]",
                    pr_event_name(key),
                    row.published_mark(),
                    row.source_mode(),
                    row.source_level(),
                    row.published_date()
                ),
            });
        }
    }
    gaps.sort_by(|left, right| left.event_key.cmp(right.event_key));
    gaps
}

pub(in crate::workbook) fn covered_events(
    pr_events: &[&'static str],
    mut covered: impl FnMut(&str) -> bool,
) -> BTreeSet<&'static str> {
    pr_events
        .iter()
        .copied()
        .filter(|event| covered(event))
        .collect()
}

pub(in crate::workbook) fn gap_lookup(gaps: &[GapLine]) -> BTreeMap<&'static str, &str> {
    gaps.iter()
        .map(|gap| (gap.event_key, gap.line.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::ReportedProfileRootV1;
    use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
    use census_domain::model::{SourceIdentity, SourceNamespace};
    use census_domain::roster_affiliation_authority::SourceRowLocator;

    fn table(rows: &[(&str, &str, &str, &str, &str)]) -> ReportedProfilePrTable {
        let root =
            ReportedProfileRootV1::journal(&"ab".repeat(32), 128).expect("journal root builds");
        ReportedProfilePrTable {
            source: SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9".to_owned())
                .with_url("https://al.milesplit.com/athletes/9".to_owned()),
            root,
            profile_url: "https://al.milesplit.com/athletes/9".to_owned(),
            body_sha256: "abc123".to_owned(),
            acquired_at: chrono::DateTime::parse_from_rfc3339("2026-10-26T00:00:00Z")
                .expect("timestamp parses")
                .with_timezone(&chrono::Utc),
            interpretation: "milesplit_public_profile_declared_pr_table_v1".to_owned(),
            rows: rows
                .iter()
                .enumerate()
                .map(|(ordinal, (event, mark, date, mode, level))| {
                    SourceProfilePrV1::replayed(
                        (*event).to_owned(),
                        (*mark).to_owned(),
                        (*date).to_owned(),
                        (*mode).to_owned(),
                        (*level).to_owned(),
                        SourceRowLocator::new(u32::try_from(ordinal).expect("ordinal fits"), 0, 10),
                    )
                })
                .collect(),
        }
    }

    #[test]
    fn unmapped_events_stay_out_and_covered_events_are_skipped() {
        let tables = [table(&[
            ("High Jump", "5-0", "May 2, 2026", "outdoor", "all"),
            ("60m Dash", "7.5", "Apr 1, 2026", "indoor", "all"),
            ("High Jump", "5-1", "Jun 1, 2026", "outdoor", "all"),
        ])];
        let refs: Vec<&ReportedProfilePrTable> = tables.iter().collect();
        let covered: BTreeSet<&str> = ["Track100m"].into_iter().collect();
        let gaps = gap_lines(&covered, &refs);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].event_key, "HighJump");
        assert!(gaps[0].line.contains("High Jump 5-0"));
        assert!(gaps[0].line.contains("source-declared profile"));
    }

    #[test]
    fn covered_gaps_disappear() {
        let tables = [table(&[(
            "High Jump",
            "5-0",
            "May 2, 2026",
            "outdoor",
            "all",
        )])];
        let refs: Vec<&ReportedProfilePrTable> = tables.iter().collect();
        let covered: BTreeSet<&str> = ["HighJump"].into_iter().collect();
        assert!(gap_lines(&covered, &refs).is_empty());
    }
}
