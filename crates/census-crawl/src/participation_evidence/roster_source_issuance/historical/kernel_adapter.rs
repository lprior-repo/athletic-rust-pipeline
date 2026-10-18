use crate::participation_evidence::{bound, invalid};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError, StoreResult};
use sha2::{Digest, Sha256};

pub(in super::super) mod context;

#[derive(Clone, Copy)]
pub(in super::super) enum KernelEpoch {
    FullV6,
    RowLocalV7,
}

#[must_use]
pub(in super::super) struct CheckedKernelReplayV1 {
    digest: &'static str,
    bytes: u64,
}

impl CheckedKernelReplayV1 {
    pub(in super::super) fn require_reference(&self, digest: &str, bytes: u64) -> StoreResult<()> {
        if self.digest != digest || self.bytes != bytes {
            return Err(invalid("historical semantic kernel reference mismatch"));
        }
        Ok(())
    }
}

struct OriginalKernel {
    digest: &'static str,
    hash: [u8; 32],
}

const FULL: OriginalKernel = OriginalKernel {
    digest: "71ef18570ab2683c9f498e2f66983d6a782900fd2647eff4101864b782ecaa46",
    hash: [
        0x71, 0xef, 0x18, 0x57, 0x0a, 0xb2, 0x68, 0x3c, 0x9f, 0x49, 0x8e, 0x2f, 0x66, 0x98, 0x3d,
        0x6a, 0x78, 0x29, 0x00, 0xfd, 0x26, 0x47, 0xef, 0xf4, 0x10, 0x18, 0x64, 0xb7, 0x82, 0xec,
        0xaa, 0x46,
    ],
};
const ROW_LOCAL: OriginalKernel = OriginalKernel {
    digest: "7352af3bf097e5c91a890809b1fa52e700469b3bbccb308eb445fe49fcf48f38",
    hash: [
        0x73, 0x52, 0xaf, 0x3b, 0xf0, 0x97, 0xe5, 0xc9, 0x1a, 0x89, 0x08, 0x09, 0xb1, 0xfa, 0x52,
        0xe7, 0x00, 0x46, 0x9b, 0x3b, 0xbc, 0xcb, 0x30, 0x8e, 0xb4, 0x45, 0xfe, 0x49, 0xfc, 0xf4,
        0x8f, 0x38,
    ],
};
const FULL_V4: OriginalKernel = OriginalKernel {
    digest: "435df869dd5994102f1aee7bebafb8f1cfc3afe5d99e8c2c60ae892be14b80d2",
    hash: [
        0x43, 0x5d, 0xf8, 0x69, 0xdd, 0x59, 0x94, 0x10, 0x2f, 0x1a, 0xee, 0x7b, 0xeb, 0xaf, 0xb8,
        0xf1, 0xcf, 0xc3, 0xaf, 0xe5, 0xd9, 0x9e, 0x8c, 0x2c, 0x60, 0xae, 0x89, 0x2b, 0xe1, 0x4b,
        0x80, 0xd2,
    ],
};
const ROW_LOCAL_V5: OriginalKernel = OriginalKernel {
    digest: "60fd1056573363a7985af568568db1f9b82f4b39d38f9e3efa3034beb3c63fe0",
    hash: [
        0x60, 0xfd, 0x10, 0x56, 0x57, 0x33, 0x63, 0xa7, 0x98, 0x5a, 0xf5, 0x68, 0x56, 0x8d, 0xb1,
        0xf9, 0xb8, 0x2f, 0x4b, 0x39, 0xd3, 0x8f, 0x9e, 0x3e, 0xfa, 0x30, 0x34, 0xbe, 0xb3, 0xc6,
        0x3f, 0xe0,
    ],
};
const EXECUTING_CONFIGURATION: [u8; 32] = [
    0xf0, 0x37, 0x64, 0x64, 0xdd, 0x91, 0xb5, 0xf6, 0x3e, 0x6b, 0x6b, 0xaa, 0xb4, 0xd9, 0x93, 0xd5,
    0xd1, 0x3d, 0x7c, 0x64, 0x41, 0x4e, 0x53, 0xd2, 0x73, 0xea, 0x77, 0x7d, 0xf4, 0xc8, 0x5d, 0x73,
];

pub(in super::super) fn bind_historical_v1(
    epoch: KernelEpoch,
    retained: &[u8],
    executing: &'static str,
    executing_digest: &'static str,
    configuration: &[u8; 32],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedKernelReplayV1> {
    let result = (|| {
        bound(
            retained.len(),
            64 * 1024,
            "retained semantic kernel document",
        )?;
        grant
            .charge_steps(1)
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
        grant
            .charge_bytes(retained.len())
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
        let bytes = u64::try_from(retained.len()).map_err(|_| StoreError::CounterOverflow)?;
        if retained == executing.as_bytes() {
            return Ok(CheckedKernelReplayV1 {
                digest: executing_digest,
                bytes,
            });
        }
        let candidates = match epoch {
            KernelEpoch::FullV6 => [&FULL, &FULL_V4],
            KernelEpoch::RowLocalV7 => [&ROW_LOCAL, &ROW_LOCAL_V5],
        };
        let actual: [u8; 32] = Sha256::digest(retained).into();
        let original = candidates
            .into_iter()
            .find(|candidate| candidate.hash == actual)
            .ok_or_else(|| invalid("unsupported historical semantic kernel identity"))?;
        if configuration != &EXECUTING_CONFIGURATION {
            return Err(invalid(
                "historical semantic kernel compiler/configuration/dependencies differ",
            ));
        }
        Ok(CheckedKernelReplayV1 {
            digest: original.digest,
            bytes,
        })
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}
