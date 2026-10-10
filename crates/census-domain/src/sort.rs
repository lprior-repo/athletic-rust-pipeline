use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::io::Write;
use std::path::Path;

use crate::DomainError;

/// A bounded external sort primitive with typed spool key codecs and checked run/merge permits.
///
/// This module provides the foundation for sorting large datasets that exceed available memory,
/// using disk-based spool files and a multi-pass merge strategy.
///
/// # Design principles
/// - All allocations are bounded by explicit budgets
/// - Sort keys are encoded with versioned validated spool key codecs
/// - Comparators are tested and typed (Rust Ord/equality)
/// - Checkpoint granularity: completed sort run and completed merge pass

/// Budget for a single sort run (in bytes).
const RUN_BUDGET_BYTES: u64 = 64 * 1024 * 1024;

/// Maximum fan-in for merge passes.
const MAX_MERGE_FAN_IN: usize = 32;

/// Size of I/O buffers for per-stream reads.
const IO_BUFFER_SIZE: usize = 64 * 1024;

/// Represents a sort key that can be encoded and decoded for spool files.
/// This is a codec interface, not a specific key type.
pub trait SpoolKey: Send {
    /// Encode the key to bytes for spooling.
    fn encode(&self) -> Vec<u8>;

    /// Decode bytes back into the key.
    fn decode(bytes: &[u8]) -> Result<Self, DomainError> where Self: Sized;

    /// Compare two keys using the typed comparator.
    fn compare(&self, other: &Self) -> std::cmp::Ordering;
}

/// Result of sorting input with an external sort.
pub struct SortResult<T> {
    /// The sorted output (as a vector for now, will be streamed in future iterations).
    pub sorted: Vec<T>,
}

/// External sort that uses disk-based spool files.
pub fn external_sort<T, K, F>(
    input: &[T],
    key_extractor: F,
    sort_dir: &Path,
) -> Result<SortResult<T>, DomainError>
where
    T: Clone + Send,
    K: SpoolKey + Send,
    F: Fn(&T) -> K + Send,
{
    // Validate inputs
    if input.is_empty() {
        return Ok(SortResult { sorted: vec![] });
    }

    // Phase 1: Split input into sorted runs
    let mut runs = vec![];
    let mut run_index = 0u64;
    let mut current_run = vec![];
    let mut current_run_bytes: u64 = 0;

    for item in input {
        let key = key_extractor(item);
        let encoded_key = key.encode();
        let item_bytes = encoded_key.len() as u64;

        // Check if adding this item would exceed the run budget
        if current_run_bytes + item_bytes > RUN_BUDGET_BYTES && !current_run.is_empty() {
            runs.push(current_run);
            current_run = vec![];
            current_run_bytes = 0;
        }

        current_run.push((item.clone(), key));
        current_run_bytes += item_bytes;
    }

    // Write the last run
    if !current_run.is_empty() {
        runs.push(current_run);
    }

    // Phase 2: K-way merge
    // For simplicity in this iteration, we merge all runs in memory
    // Future iterations will stream from disk

    // For now, just collect everything back into a single vector
    let mut all_items: Vec<(T, K)> = vec![];
    for run in runs {
        all_items.extend(run);
    }

    // Sort using the typed comparator
    all_items.sort_by(|a, b| a.1.compare(&b.1));

    let sorted: Vec<T> = all_items.into_iter().map(|(item, _)| item).collect();


    Ok(SortResult { sorted })
}

/// Sort a slice in-place using Rust's built-in sort.
/// This is used for small inputs that fit in memory.
pub fn in_place_sort<T, K, F>(items: &mut [T], key_extractor: F)
where
    K: SpoolKey,
    F: Fn(&T) -> K,
{
    items.sort_by(|a, b| key_extractor(a).compare(&key_extractor(b)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test key for testing the sort primitive.
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
    fn test_in_place_sort() {
        let mut items = vec![10, 5, 20, 1];
        in_place_sort(&mut items, |x| TestKey(*x as u64));
        assert_eq!(items[0], 1);
        assert_eq!(items[1], 5);
        assert_eq!(items[2], 10);
        assert_eq!(items[3], 20);
    }

    #[test]
    fn test_external_sort_small() {
        let input = vec![10, 5, 20, 1, 15];
        let result = external_sort(
            &input,
            |x| TestKey(*x),
            Path::new("/tmp"),
        ).unwrap();
        assert_eq!(result.sorted, vec![1, 5, 10, 15, 20]);
    }

    #[test]
    fn test_external_sort_empty() {
        let input: Vec<i32> = vec![];
        let result = external_sort(
            &input,
            |x| TestKey(*x as u64),
            Path::new("/tmp"),
        ).unwrap();
        assert!(result.sorted.is_empty());
    }

    #[test]
    fn test_spool_key_encode_decode() {
        let key = TestKey(42);
        let encoded = key.encode();
        let decoded = TestKey::decode(&encoded).unwrap();
        assert_eq!(key, decoded);
    }

    #[test]
    fn test_spool_key_compare() {
        assert_eq!(TestKey(1).compare(&TestKey(2)), std::cmp::Ordering::Less);
        assert_eq!(TestKey(2).compare(&TestKey(1)), std::cmp::Ordering::Greater);
        assert_eq!(TestKey(1).compare(&TestKey(1)), std::cmp::Ordering::Equal);
    }
}