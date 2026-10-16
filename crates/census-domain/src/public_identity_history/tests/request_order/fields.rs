use super::*;

#[test]
fn split_fields_refuse_permutations_and_report_the_owning_duplicate() -> TestResult {
    type Change = fn(&mut SplitRequest, bool);
    let changes: [(&str, Change); 5] = [
        ("revocations", |value, duplicate| {
            value.revoke_merges.swap(0, 1);
            if duplicate {
                value.revoke_merges[1] = value.revoke_merges[0];
            }
        }),
        ("unresolved bindings", |value, duplicate| {
            value.unresolved_bindings.swap(0, 1);
            if duplicate {
                value.unresolved_bindings[1] = value.unresolved_bindings[0].clone();
            }
        }),
        ("affected records", |value, duplicate| {
            value.affected_record_keys.swap(0, 1);
            if duplicate {
                value.affected_record_keys[1] = value.affected_record_keys[0].clone();
            }
        }),
        ("partition bindings", |value, duplicate| {
            let items = &mut value.partitions[0].binding_ids;
            items.swap(0, 1);
            if duplicate {
                items[1] = items[0].clone();
            }
        }),
        ("outer partitions", |value, _| value.partitions.swap(0, 1)),
    ];
    for (field, change) in changes {
        for duplicate in [false, true] {
            if duplicate && field == "outer partitions" {
                continue;
            }
            let mut split = split_request(&[&["a", "b"], &["c"]], vec![4, 5])?;
            split.unresolved_bindings = vec!["binding:d".into(), "binding:e".into()];
            let HistoryRequest::Split(mut split) =
                request_order::normalize(HistoryRequest::Split(split))?
            else {
                return Err("split normalization changed variant".into());
            };
            change(&mut split, duplicate);
            let expected = if duplicate && field != "outer partitions" {
                HistoryError::Duplicate(field)
            } else {
                HistoryError::Invalid("noncanonical request order")
            };
            check!(eq; request_order::require_canonical(&HistoryRequest::Split(split)), Err(expected));
        }
    }
    Ok(())
}

#[test]
fn merge_members_and_revocation_fields_keep_strict_ordering() -> TestResult {
    for duplicate in [false, true] {
        let mut merge_members = vec![
            PublicAthleteId::parse(SECOND)?,
            PublicAthleteId::parse(FIRST)?,
        ];
        if duplicate {
            merge_members[1] = merge_members[0].clone();
        }
        let merge = HistoryRequest::Merge {
            canonical: PublicAthleteId::parse(FIRST)?,
            members: merge_members,
            evidence: evidence("merge", CandidateEvidenceKind::SamePerson, &["a"]),
        };
        let expected = if duplicate {
            HistoryError::Duplicate("merge members")
        } else {
            HistoryError::Invalid("noncanonical request order")
        };
        check!(eq; request_order::require_canonical(&merge), Err(expected));
        for field in ["revocations", "affected records"] {
            let mut revocation = RevocationRequest {
                canonical: PublicAthleteId::parse(FIRST)?,
                revoke_merges: vec![4, 5],
                affected_record_keys: vec!["performance:1".into(), "performance:2".into()],
                evidence: evidence("correction", CandidateEvidenceKind::Correction, &["a"]),
            };
            if field == "revocations" {
                revocation.revoke_merges.swap(0, 1);
                if duplicate {
                    revocation.revoke_merges[1] = revocation.revoke_merges[0];
                }
            } else {
                revocation.affected_record_keys.swap(0, 1);
                if duplicate {
                    revocation.affected_record_keys[1] = revocation.affected_record_keys[0].clone();
                }
            }
            let expected = if duplicate {
                HistoryError::Duplicate(field)
            } else {
                HistoryError::Invalid("noncanonical request order")
            };
            check!(eq; request_order::require_canonical(&HistoryRequest::Revoke(revocation)), Err(expected));
        }
    }
    Ok(())
}
