use sha2::{Digest, Sha256};

use super::SealEvidence;

pub(super) fn render(evidence: &SealEvidence) -> String {
    let counts = evidence.counts;
    let inputs = evidence.retained.gap_digest_inputs();
    let tallies = inputs
        .gaps
        .iter()
        .map(|gap| format!("{}:{}:{}", gap.class, gap.unit, gap.count))
        .collect::<Vec<_>>()
        .join("|");
    let mut workbook_digests = evidence.workbook.digests.clone();
    workbook_digests.sort();
    let workbook_digests = workbook_digests.join("|");
    let silent_sources = inputs.silent_sources.join("|");
    let source_failures = match inputs.source_failures {
        Some(count) => count.to_string(),
        None => "unmeasured".to_string(),
    };
    let run_identity = match evidence.run {
        Some(run) => format!("{}-{}", run.season().get(), run.revision()),
        None => "unbound".to_string(),
    };
    let rendered = format!(
            "census-seal-v7\nrun_identity={run_identity}\njurisdiction_buckets={}\nschools={}\nmeets={}\nathletes={}\nco2027={}\ncohort_performances={}\ncoaches={}\nconflicts={}\naccess_conditions={}\nblocked_hosts={}\nthrottled_hosts={}\nsilent_sources={silent_sources}\nsource_failures={source_failures}\nobservations={}\ncalculations={}\nworkbook_rows={}\nworkbook_sheets={}\nworkbook_sha256={workbook_digests}\ngaps={tallies}\n",
            counts.jurisdiction_buckets,
            counts.schools,
            counts.meets,
            counts.athletes,
            counts.class_of_2027,
            counts.cohort_performances,
            counts.coaches,
            evidence.retained.conflicts,
            evidence.retained.access_conditions,
            evidence.retained.blocked_hosts,
            evidence.retained.throttled_hosts,
            evidence.retained.observations,
            evidence.retained.calculations,
            evidence.workbook.rows,
            evidence.workbook.sheets,
        );
    let mut hasher = Sha256::new();
    hasher.update(rendered.as_bytes());
    let bytes = hasher.finalize();
    let mut hex = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes.iter() {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::super::{GapTally, RetainedFindings, SealEvidence};
    use super::render;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn retained() -> RetainedFindings {
        RetainedFindings {
            gaps: vec![
                GapTally {
                    class: "missing_coach".to_string(),
                    unit: "athletes".to_string(),
                    count: 264_452,
                },
                GapTally {
                    class: "missing_profile".to_string(),
                    unit: "athletes".to_string(),
                    count: 13_202,
                },
            ],
            silent_sources: vec!["wayzata_mn".to_string(), "wiaa_results_wi".to_string()],
            source_failures: Some(4),
            ..Default::default()
        }
    }

    fn evidence_with(retained: RetainedFindings) -> SealEvidence {
        SealEvidence {
            retained,
            ..Default::default()
        }
    }

    #[test]
    fn dropping_a_gap_changes_the_published_inputs_and_the_digest() -> TestResult {
        let base = evidence_with(retained());
        let mut dropped = retained();
        dropped.gaps.truncate(1);
        let without = evidence_with(dropped);
        check!(ne;
            base.retained.gap_digest_inputs(),
            without.retained.gap_digest_inputs(),
            "removing a gap tally must move the published gap inputs"
        );
        check!(ne;
            render(&base),
            render(&without),
            "removing a gap tally must move the seal digest"
        );
        Ok(())
    }

    #[test]
    fn dropping_a_silent_source_changes_the_published_inputs_and_the_digest() -> TestResult {
        let base = evidence_with(retained());
        let mut dropped = retained();
        dropped.silent_sources.truncate(1);
        let without = evidence_with(dropped);
        check!(ne;
            base.retained.gap_digest_inputs(),
            without.retained.gap_digest_inputs(),
            "removing a silent source must move the published gap inputs"
        );
        check!(ne;
            render(&base),
            render(&without),
            "removing a silent source must move the seal digest"
        );
        Ok(())
    }

    #[test]
    fn dropping_the_source_failure_count_changes_the_published_inputs_and_the_digest() -> TestResult
    {
        let base = evidence_with(retained());
        let mut dropped = retained();
        dropped.source_failures = None;
        let without = evidence_with(dropped);
        check!(ne;
            base.retained.gap_digest_inputs(),
            without.retained.gap_digest_inputs(),
            "unmeasuring the failure count must move the published gap inputs"
        );
        check!(ne;
            render(&base),
            render(&without),
            "unmeasuring the failure count must move the seal digest"
        );
        Ok(())
    }

    #[test]
    fn the_published_inputs_carry_the_digest_canonical_form() -> TestResult {
        let mut shuffled = retained();
        shuffled.gaps.reverse();
        shuffled.silent_sources.reverse();
        shuffled.silent_sources.push("wayzata_mn".to_string());
        let published = shuffled.gap_digest_inputs();
        check!(eq; published.gaps, retained().gaps);
        check!(eq;
            published.silent_sources,
            vec!["wayzata_mn".to_string(), "wiaa_results_wi".to_string()]
        );
        check!(eq; published.source_failures, Some(4));
        let base = evidence_with(retained());
        let canonical = evidence_with(RetainedFindings {
            gaps: published.gaps,
            silent_sources: published.silent_sources,
            source_failures: published.source_failures,
            ..Default::default()
        });
        check!(eq; render(&base), render(&canonical));
        Ok(())
    }
}
