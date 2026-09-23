//! Rendering the seal digest: the bytes that identify one census.
//!
//! Deterministic by construction. The gap tallies are sorted before rendering, so gap order is not
//! evidence; the open-work counts are `Some(0)` wherever a seal exists, because a `None` (unmeasured)
//! keeps an acceptance item open and no seal can carry one; and the workbook's own sha256 is
//! included, so two exports with the same row count cannot share a seal.
//!
//! The version prefix is `v3`: v2 hashed the access-condition count of a blocked or throttled host
//! under the name `retry_exhausted`, which said it counted per-attempt exhaustion; v1 hashed
//! `source_failures` as a bare integer, and that field is now tri-state. A digest written by an older
//! seal cannot be compared with a current one, and the prefix is what makes that visible instead of
//! silently producing a different number for the same evidence.

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
    // Tri-state: an unmeasured count must not hash like a measured zero, or two seals with
    // different evidence would share a digest.
    let source_failures = match evidence.retained.source_failures {
        Some(count) => count.to_string(),
        None => "unmeasured".to_string(),
    };
    let rendered = format!(
            "census-seal-v3\njurisdictions={}\nschools={}\nmeets={}\nathletes={}\nco2027={}\nperformances={}\ncoaches={}\nconflicts={}\naccess_conditions={}\nblocked_hosts={}\nthrottled_hosts={}\nsource_failures={source_failures}\nobservations={}\ncalculations={}\nworkbook_rows={}\nworkbook_sheets={}\nworkbook_sha256={workbook_digests}\ngaps={tallies}\n",
            counts.jurisdictions,
            counts.schools,
            counts.meets,
            counts.athletes,
            counts.class_of_2027,
            counts.performances,
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
