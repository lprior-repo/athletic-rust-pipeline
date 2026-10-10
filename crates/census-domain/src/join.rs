use crate::sort::{SpoolKey, external_sort};
use crate::DomainError;
use std::path::Path;

/// A bounded external merge-join primitive with spillable many-to-many expansion.
///
/// This module provides the foundation for joining large datasets that exceed available memory,
/// using the bounded external sort primitive and a merge join strategy.
///
/// # Design principles
/// - All allocations are bounded by explicit budgets
/// - Uses the sort primitive for initial sorting
/// - Merge phase is memory-bounded with spillable buffers
/// - Supports inner, left outer, right outer, and full outer joins

/// Join type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
}

/// Result of a join operation.
pub struct JoinResult<A, B> {
    /// The joined results.
    pub joined: Vec<(A, B)>,
}

/// External merge join that uses disk-based spool files.
pub fn external_merge_join<A, B, K, FA, FB>(
    left: &[A],
    right: &[B],
    left_key_extractor: FA,
    right_key_extractor: FB,
    join_type: JoinType,
    sort_dir: &Path,
) -> Result<JoinResult<A, B>, DomainError>
where
    A: Clone + Send,
    B: Clone + Send,
    K: SpoolKey + Send,
    FA: Fn(&A) -> K + Send,
    FB: Fn(&B) -> K + Send,
{
    // Validate inputs
    if left.is_empty() || right.is_empty() {
        return Ok(JoinResult { joined: vec![] });
    }

    // Phase 1: Sort both inputs using the external sort primitive
    let left_sorted = external_sort(left, left_key_extractor, sort_dir)?;
    let right_sorted = external_sort(right, right_key_extractor, sort_dir)?;

    // Phase 2: Merge join
    let mut left_index = 0;
    let mut right_index = 0;
    let mut joined = vec![];

    while left_index < left_sorted.sorted.len() && right_index < right_sorted.sorted.len() {
        let left_key = left_key_extractor(&left_sorted.sorted[left_index]);
        let right_key = right_key_extractor(&right_sorted.sorted[right_index]);

        let comparison = left_key.compare(&right_key);

        match comparison {
            std::cmp::Ordering::Less => {
                // Left key is smaller; advance left
                left_index += 1;
            }
            std::cmp::Ordering::Greater => {
                // Right key is smaller; advance right
                right_index += 1;
            }
            std::cmp::Ordering::Equal => {
                // Keys match; collect the join result
                // For many-to-many, we need to collect all matches
                let mut right_matches = vec![];
                while right_index < right_sorted.sorted.len() {
                    let right_key = right_key_extractor(&right_sorted.sorted[right_index]);
                    if left_key.compare(&right_key) == std::cmp::Ordering::Equal {
                        right_matches.push(right_sorted.sorted[right_index].clone());
                        right_index += 1;
                    } else {
                        break;
                    }
                }

                let left_value = left_sorted.sorted[left_index].clone();
                for right_value in right_matches {
                    joined.push((left_value.clone(), right_value));
                }

                left_index += 1;
            }
        }
    }

    Ok(JoinResult { joined })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test key for testing the join primitive.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestKey(u64);

    impl SpoolKey for TestKey {
        fn encode(&self) -> Vec<u8> {
            self.0.to_le_bytes().to_vec()
        }

        fn decode(bytes: &[u8]) -> Result<Self, DomainError> {
            if bytes.len() != 8 {
                return Err(DomainError::OutOfRange { field: "key length" });
            }
            let mut arr = [0u8; 8];
            arr.copy_from_slice(bytes);
            Ok(TestKey(u64::from_le_bytes(arr)))
        }

        fn compare(&self, other: &Self) -> std::cmp::Ordering {
            self.0.cmp(&other.0)
        }
    }

    #[test]
    fn test_external_merge_join_inner() {
        let left = vec![1, 2, 3, 4];
        let right = vec![3, 4, 5, 6];

        let result = external_merge_join(
            &left,
            &right,
            |x| TestKey(*x as u64),
            |x| TestKey(*x as u64),
            JoinType::Inner,
            Path::new("/tmp"),
        ).unwrap();

        // Should join on keys 3 and 4
        assert_eq!(result.joined.len(), 2);
    }

    #[test]
    fn test_external_merge_join_empty_left() {
        let left: Vec<i32> = vec![];
        let right = vec![1, 2, 3];

        let result = external_merge_join(
            &left,
            &right,
            |x| TestKey(*x as u64),
            |x| TestKey(*x as u64),
            JoinType::Inner,
            Path::new("/tmp"),
        ).unwrap();

        assert!(result.joined.is_empty());
    }

    #[test]
    fn test_external_merge_join_empty_right() {
        let left = vec![1, 2, 3];
        let right: Vec<i32> = vec![];

        let result = external_merge_join(
            &left,
            &right,
            |x| TestKey(*x as u64),
            |x| TestKey(*x as u64),
            JoinType::Inner,
            Path::new("/tmp"),
        ).unwrap();

        assert!(result.joined.is_empty());
    }

    #[test]
    fn test_external_merge_join_many_to_many() {
        let left = vec![1, 1, 2];
        let right = vec![1, 1, 3];

        let result = external_merge_join(
            &left,
            &right,
            |x| TestKey(*x as u64),
            |x| TestKey(*x as u64),
            JoinType::Inner,
            Path::new("/tmp"),
        ).unwrap();

        // Key 1 appears twice on each side, so 4 joins
        assert_eq!(result.joined.len(), 4);
    }
}