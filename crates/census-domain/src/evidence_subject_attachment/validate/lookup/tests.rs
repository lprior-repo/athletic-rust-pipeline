use super::*;
use crate::evidence_subject_attachment::AttachmentObjectReferenceV1;
use crate::public_identity_history::ReplayLimits;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn digest(value: usize) -> TestResult<AttachmentDigestV1> {
    Ok(AttachmentDigestV1::parse(&format!("{value:064x}"))?)
}

fn rows(count: usize) -> TestResult<Vec<AttachmentObjectReferenceV1>> {
    (0..count)
        .map(|position| {
            Ok(AttachmentObjectReferenceV1 {
                digest: digest(
                    position
                        .checked_add(1)
                        .and_then(|n| n.checked_mul(2))
                        .ok_or("fixture overflow")?,
                )?,
                bytes: u64::try_from(position.checked_add(17).ok_or("fixture overflow")?)?,
            })
        })
        .collect()
}

#[test]
fn ordered_lookup_matches_complete_linear_rows_for_all_bounded_cardinalities() -> TestResult {
    for count in 0..=64 {
        let rows = rows(count)?;
        for value in 0..=130 {
            let expected = digest(value)?;
            let linear = rows.iter().find(|row| row.digest == expected);
            let mut grant = Grant::new();
            let found = find(&rows, &expected, |row| &row.digest, &mut grant)?;
            check!(eq; found, linear);
            if let (Some(found), Some(linear)) = (found, linear) {
                check!(std::ptr::eq(found, linear));
            }
            check!(grant.usage().traversal_steps() <= 7);
            check!(eq; grant.usage().encoded_bytes(), 0);
        }
    }
    Ok(())
}

#[test]
fn midpoint_work_matches_fixed_rank_table_including_both_absent_boundaries() -> TestResult {
    let rows = rows(8)?;
    for (value, comparisons) in [
        (0, 4),
        (2, 4),
        (4, 3),
        (6, 2),
        (8, 3),
        (10, 1),
        (12, 3),
        (14, 2),
        (16, 3),
        (18, 3),
    ] {
        let mut grant = Grant::new();
        find(&rows, &digest(value)?, |row| &row.digest, &mut grant)?;
        check!(eq; grant.usage().traversal_steps(), comparisons);
    }
    Ok(())
}

#[test]
fn exact_lookup_work_succeeds_and_one_less_preserves_sticky_capacity() -> TestResult {
    let rows = rows(8)?;
    let expected = digest(2)?;
    let mut exact = Grant::new();
    exact.charge_steps(ReplayLimits::SMALL.traversal_steps - 4)?;
    check!(eq; find(&rows, &expected, |row| &row.digest, &mut exact)?, rows.first());
    check!(eq; exact.usage().traversal_steps(), ReplayLimits::SMALL.traversal_steps);
    let mut short = Grant::new();
    short.charge_steps(ReplayLimits::SMALL.traversal_steps - 3)?;
    check!(matches!(
        find(&rows, &expected, |row| &row.digest, &mut short),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8193,
            available: 8192
        })
    ));
    let usage = short.usage();
    check!(matches!(
        find(&rows, &expected, |row| &row.digest, &mut short),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; short.usage(), usage);
    Ok(())
}

#[test]
fn exhausted_admission_precedes_key_access_and_empty_lookup_cannot_rehabilitate_it() -> TestResult {
    struct Probe {
        digest: AttachmentDigestV1,
        visited: std::cell::Cell<bool>,
    }
    let rows = [Probe {
        digest: digest(2)?,
        visited: std::cell::Cell::new(false),
    }];
    let mut grant = Grant::new();
    grant.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    check!(matches!(
        find(
            &rows,
            &digest(2)?,
            |row| {
                row.visited.set(true);
                &row.digest
            },
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8193,
            available: 8192
        })
    ));
    check!(!rows.first().ok_or("probe absent")?.visited.get());
    check!(matches!(
        find(
            &[] as &[AttachmentDigestV1],
            &digest(2)?,
            |row| row,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
