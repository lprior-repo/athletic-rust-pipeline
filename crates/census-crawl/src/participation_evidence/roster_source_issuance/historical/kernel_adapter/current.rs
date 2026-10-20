use super::execution::RAW_VALUE_PORTABLE_CONFIGURATION;
use super::{CheckedKernelReplayV1, KernelEpoch};
use crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2;

const FULL_NATIVE: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/roster-semantic-kernel-v12-native.json"
));
const ROW_LOCAL_NATIVE: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/roster-semantic-kernel-v13-native.json"
));
const FULL_NATIVE_DIGEST: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/roster-semantic-kernel-v12-native.sha256"
));
const ROW_LOCAL_NATIVE_DIGEST: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/roster-semantic-kernel-v13-native.sha256"
));

pub(super) fn native(
    epoch: KernelEpoch,
    retained: &[u8],
    configuration: &[u8; 32],
    bytes: u64,
) -> Option<CheckedKernelReplayV1> {
    if configuration != &RAW_VALUE_PORTABLE_CONFIGURATION {
        return None;
    }
    let (expected, digest) = match epoch {
        KernelEpoch::Full => (FULL_NATIVE, FULL_NATIVE_DIGEST),
        KernelEpoch::RowLocal => (ROW_LOCAL_NATIVE, ROW_LOCAL_NATIVE_DIGEST),
    };
    if retained != expected {
        return None;
    }
    Some(CheckedKernelReplayV1 {
        digest,
        bytes,
        semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2,
    })
}
