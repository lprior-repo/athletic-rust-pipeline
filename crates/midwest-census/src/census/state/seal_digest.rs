//! Rendering the seal digest: the bytes that identify one census.
//!
//! Deterministic by construction. The gap tallies are sorted before rendering, so gap order is not
//! evidence; the open-work counts are all zero wherever a seal exists; and the workbook's own sha256
//! is included, so two exports with the same row count cannot share a seal.

use sha2::{Digest, Sha256};

use super::SealEvidence;

/// A digest over the certified numbers, the retained findings and the workbook bytes.
pub(super) fn render(evidence: &SealEvidence) -> String {
    let counts = evidence.counts;
    let mut gaps = evidence.retained.gaps.clone();
    gaps.sort();
    let tallies = gaps
        .iter()
        .map(|gap| format!("{}:{}:{}", gap.class, gap.unit, gap.count))
        .collect::<Vec<_>>()
        .join("|");
    // The workbook's own bytes belong in the digest: two exports with the same row count are not
    // the same artifact, and a seal that cannot tell them apart certifies neither.
    let mut workbook_digests = evidence.workbook.digests.clone();
    workbook_digests.sort();
    let workbook_digests = workbook_digests.join("|");
    let rendered = format!(
            "census-seal-v1\njurisdictions={}\nschools={}\nmeets={}\nathletes={}\nco2027={}\nperformances={}\ncoaches={}\nconflicts={}\nretry_exhausted={}\nsource_failures={}\nobservations={}\ncalculations={}\nworkbook_rows={}\nworkbook_sheets={}\nworkbook_sha256={workbook_digests}\ngaps={tallies}\n",
            counts.jurisdictions,
            counts.schools,
            counts.meets,
            counts.athletes,
            counts.class_of_2027,
            counts.performances,
            counts.coaches,
            evidence.retained.conflicts,
            evidence.retained.retry_exhausted,
            evidence.retained.source_failures,
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
