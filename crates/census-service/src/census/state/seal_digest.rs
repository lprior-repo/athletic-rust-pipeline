use sha2::{Digest, Sha256};

use super::SealEvidence;

pub(super) fn render(evidence: &SealEvidence) -> String {
    let counts = evidence.counts;
    let mut gaps = evidence.retained.gaps.clone();
    gaps.sort();
    let tallies = gaps
        .iter()
        .map(|gap| format!("{}:{}:{}", gap.class, gap.unit, gap.count))
        .collect::<Vec<_>>()
        .join("|");
    let mut workbook_digests = evidence.workbook.digests.clone();
    workbook_digests.sort();
    let workbook_digests = workbook_digests.join("|");
    let silent_sources = {
        let mut names = evidence.retained.silent_sources.clone();
        names.sort();
        names.dedup();
        names.join("|")
    };
    let source_failures = match evidence.retained.source_failures {
        Some(count) => count.to_string(),
        None => "unmeasured".to_string(),
    };
    let rendered = format!(
            "census-seal-v6\njurisdiction_buckets={}\nschools={}\nmeets={}\nathletes={}\nco2027={}\ncohort_performances={}\ncoaches={}\nconflicts={}\naccess_conditions={}\nblocked_hosts={}\nthrottled_hosts={}\nsilent_sources={silent_sources}\nsource_failures={source_failures}\nobservations={}\ncalculations={}\nworkbook_rows={}\nworkbook_sheets={}\nworkbook_sha256={workbook_digests}\ngaps={tallies}\n",
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
