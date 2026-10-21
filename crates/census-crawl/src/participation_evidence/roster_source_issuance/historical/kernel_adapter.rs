use crate::participation_evidence::{bound, invalid};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError, StoreResult};
use sha2::{Digest, Sha256};

use crate::participation_evidence::provider_school::{
    PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1, PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2,
};
pub(in super::super) mod context;
mod current;
mod execution;
mod grant4g;
mod member;

#[derive(Clone, Copy)]
pub(in super::super) enum KernelEpoch {
    #[cfg(test)]
    Full,
    RowLocal,
}

#[must_use]
pub(in super::super) struct CheckedKernelReplayV1 {
    #[cfg(test)]
    digest: &'static str,
    #[cfg(test)]
    bytes: u64,
    semantic_contract: &'static str,
}

impl CheckedKernelReplayV1 {
    #[cfg(test)]
    pub(in super::super) fn require_reference(&self, digest: &str, bytes: u64) -> StoreResult<()> {
        if self.digest != digest || self.bytes != bytes {
            return Err(invalid("historical semantic kernel reference mismatch"));
        }
        Ok(())
    }

    pub(in super::super) fn semantic_contract(&self) -> &'static str {
        self.semantic_contract
    }

    pub(in super::super) fn require_semantic_contract(&self, contract: &str) -> StoreResult<()> {
        if contract != self.semantic_contract {
            return Err(invalid("historical semantic kernel owner policy mismatch"));
        }
        Ok(())
    }
}

struct OriginalKernel {
    #[cfg(test)]
    digest: &'static str,
    hash: [u8; 32],
    configuration: &'static [u8; 32],
    semantic_contract: &'static str,
}

#[cfg(test)]
const FULL: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "71ef18570ab2683c9f498e2f66983d6a782900fd2647eff4101864b782ecaa46",
    hash: [
        0x71, 0xef, 0x18, 0x57, 0x0a, 0xb2, 0x68, 0x3c, 0x9f, 0x49, 0x8e, 0x2f, 0x66, 0x98, 0x3d,
        0x6a, 0x78, 0x29, 0x00, 0xfd, 0x26, 0x47, 0xef, 0xf4, 0x10, 0x18, 0x64, 0xb7, 0x82, 0xec,
        0xaa, 0x46,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
const ROW_LOCAL: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "7352af3bf097e5c91a890809b1fa52e700469b3bbccb308eb445fe49fcf48f38",
    hash: [
        0x73, 0x52, 0xaf, 0x3b, 0xf0, 0x97, 0xe5, 0xc9, 0x1a, 0x89, 0x08, 0x09, 0xb1, 0xfa, 0x52,
        0xe7, 0x00, 0x46, 0x9b, 0x3b, 0xbc, 0xcb, 0x30, 0x8e, 0xb4, 0x45, 0xfe, 0x49, 0xfc, 0xf4,
        0x8f, 0x38,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
#[cfg(test)]
const FULL_V4: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "435df869dd5994102f1aee7bebafb8f1cfc3afe5d99e8c2c60ae892be14b80d2",
    hash: [
        0x43, 0x5d, 0xf8, 0x69, 0xdd, 0x59, 0x94, 0x10, 0x2f, 0x1a, 0xee, 0x7b, 0xeb, 0xaf, 0xb8,
        0xf1, 0xcf, 0xc3, 0xaf, 0xe5, 0xd9, 0x9e, 0x8c, 0x2c, 0x60, 0xae, 0x89, 0x2b, 0xe1, 0x4b,
        0x80, 0xd2,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
const ROW_LOCAL_V5: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "60fd1056573363a7985af568568db1f9b82f4b39d38f9e3efa3034beb3c63fe0",
    hash: [
        0x60, 0xfd, 0x10, 0x56, 0x57, 0x33, 0x63, 0xa7, 0x98, 0x5a, 0xf5, 0x68, 0x56, 0x8d, 0xb1,
        0xf9, 0xb8, 0x2f, 0x4b, 0x39, 0xd3, 0x8f, 0x9e, 0x3e, 0xfa, 0x30, 0x34, 0xbe, 0xb3, 0xc6,
        0x3f, 0xe0,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
#[cfg(test)]
const FULL_V6: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "ba3d52bbc2673fc3f386815a39ed09702b5e2c68b528ca8eecf630d5f45ea8fd",
    hash: [
        0xba, 0x3d, 0x52, 0xbb, 0xc2, 0x67, 0x3f, 0xc3, 0xf3, 0x86, 0x81, 0x5a, 0x39, 0xed, 0x09,
        0x70, 0x2b, 0x5e, 0x2c, 0x68, 0xb5, 0x28, 0xca, 0x8e, 0xec, 0xf6, 0x30, 0xd5, 0xf4, 0x5e,
        0xa8, 0xfd,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
const ROW_LOCAL_V7: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "b49bfad4374e7a664c08977bc62428e40e8c8e0063d33235796e5eb3b03c2f65",
    hash: [
        0xb4, 0x9b, 0xfa, 0xd4, 0x37, 0x4e, 0x7a, 0x66, 0x4c, 0x08, 0x97, 0x7b, 0xc6, 0x24, 0x28,
        0xe4, 0x0e, 0x8c, 0x8e, 0x00, 0x63, 0xd3, 0x32, 0x35, 0x79, 0x6e, 0x5e, 0xb3, 0xb0, 0x3c,
        0x2f, 0x65,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
const EXECUTING_CONFIGURATION: [u8; 32] = [
    0x9e, 0x7f, 0x86, 0xbb, 0x6f, 0x37, 0x45, 0x09, 0xc1, 0x44, 0xcc, 0xf9, 0x9f, 0xd3, 0xb2, 0x74,
    0x1b, 0x55, 0x12, 0x94, 0x78, 0x40, 0x97, 0x32, 0x99, 0xfe, 0x88, 0x7c, 0x3d, 0xb4, 0x12, 0x10,
];
const PORTABLE_EXECUTING_CONFIGURATION: [u8; 32] = [
    0xf4, 0xfe, 0x30, 0xb9, 0xc6, 0xe1, 0x5d, 0xce, 0x5d, 0xe2, 0x0c, 0x69, 0x8f, 0xbd, 0xd2, 0x2d,
    0x2e, 0xcc, 0xf6, 0x3d, 0xe8, 0x6d, 0x82, 0x0c, 0x3a, 0x88, 0x7e, 0x73, 0x39, 0x3d, 0xe3, 0x87,
];
const ALTERNATE_CONFIGURATION: [u8; 32] = [
    0x91, 0x91, 0x04, 0x92, 0x40, 0x74, 0xec, 0x58, 0xa3, 0xa2, 0x1b, 0x1d, 0x0a, 0xd4, 0x67, 0xb8,
    0xc6, 0x3c, 0x9f, 0x9c, 0x15, 0x5f, 0x27, 0xa4, 0xd3, 0x20, 0x4c, 0x14, 0x74, 0x41, 0xc9, 0x6f,
];
#[cfg(test)]
const FULL_V8: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "8a4c9f7f31199c9f856ed43f9905ed01741cfa506707c8cd472cc6032b331b6f",
    hash: [
        0x8a, 0x4c, 0x9f, 0x7f, 0x31, 0x19, 0x9c, 0x9f, 0x85, 0x6e, 0xd4, 0x3f, 0x99, 0x05, 0xed,
        0x01, 0x74, 0x1c, 0xfa, 0x50, 0x67, 0x07, 0xc8, 0xcd, 0x47, 0x2c, 0xc6, 0x03, 0x2b, 0x33,
        0x1b, 0x6f,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
const ROW_LOCAL_V9: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "94ad0cb66c947e1fdc1efd31cbd99c7d1aac3bbc580bf1e81debfd16bf188505",
    hash: [
        0x94, 0xad, 0x0c, 0xb6, 0x6c, 0x94, 0x7e, 0x1f, 0xdc, 0x1e, 0xfd, 0x31, 0xcb, 0xd9, 0x9c,
        0x7d, 0x1a, 0xac, 0x3b, 0xbc, 0x58, 0x0b, 0xf1, 0xe8, 0x1d, 0xeb, 0xfd, 0x16, 0xbf, 0x18,
        0x85, 0x05,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
#[cfg(test)]
const FULL_V8_ALTERNATE: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "9a4b8f629bfa43a1e706b8acc19498cf201876c583fec611355fbe006c4e97c3",
    hash: [
        0x9a, 0x4b, 0x8f, 0x62, 0x9b, 0xfa, 0x43, 0xa1, 0xe7, 0x06, 0xb8, 0xac, 0xc1, 0x94, 0x98,
        0xcf, 0x20, 0x18, 0x76, 0xc5, 0x83, 0xfe, 0xc6, 0x11, 0x35, 0x5f, 0xbe, 0x00, 0x6c, 0x4e,
        0x97, 0xc3,
    ],
    configuration: &ALTERNATE_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
const ROW_LOCAL_V9_ALTERNATE: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "93351cb0eeff6d532402e831d6f61143d1b1f9e31e0a77a0629094ea46b5d34b",
    hash: [
        0x93, 0x35, 0x1c, 0xb0, 0xee, 0xff, 0x6d, 0x53, 0x24, 0x02, 0xe8, 0x31, 0xd6, 0xf6, 0x11,
        0x43, 0xd1, 0xb1, 0xf9, 0xe3, 0x1e, 0x0a, 0x77, 0xa0, 0x62, 0x90, 0x94, 0xea, 0x46, 0xb5,
        0xd3, 0x4b,
    ],
    configuration: &ALTERNATE_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1,
};
#[cfg(test)]
const QUALIFIED_FULL_V10: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "35695fab34f7e130fe3c3560e0568edf0897b837488ee7761864103176d6dd1b",
    hash: [
        0x35, 0x69, 0x5f, 0xab, 0x34, 0xf7, 0xe1, 0x30, 0xfe, 0x3c, 0x35, 0x60, 0xe0, 0x56, 0x8e,
        0xdf, 0x08, 0x97, 0xb8, 0x37, 0x48, 0x8e, 0xe7, 0x76, 0x18, 0x64, 0x10, 0x31, 0x76, 0xd6,
        0xdd, 0x1b,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2,
};
const QUALIFIED_ROW_LOCAL_V11: OriginalKernel = OriginalKernel {
    #[cfg(test)]
    digest: "7c2fb5be2b9948e8df8c829fd9ac09ac7524d693ca980b3df1c81f5908ad858b",
    hash: [
        0x7c, 0x2f, 0xb5, 0xbe, 0x2b, 0x99, 0x48, 0xe8, 0xdf, 0x8c, 0x82, 0x9f, 0xd9, 0xac, 0x09,
        0xac, 0x75, 0x24, 0xd6, 0x93, 0xca, 0x98, 0x0b, 0x3d, 0xf1, 0xc8, 0x1f, 0x59, 0x08, 0xad,
        0x85, 0x8b,
    ],
    configuration: &EXECUTING_CONFIGURATION,
    semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2,
};

fn predecessors(epoch: KernelEpoch) -> [&'static OriginalKernel; 16] {
    match epoch {
        #[cfg(test)]
        KernelEpoch::Full => [
            &FULL,
            &FULL_V4,
            &FULL_V6,
            &FULL_V8,
            &FULL_V8_ALTERNATE,
            &QUALIFIED_FULL_V10,
            &grant4g::FULL_NATIVE,
            &grant4g::FULL_PORTABLE,
            &member::FULL_NATIVE,
            &member::FULL_RAW_VALUE_NATIVE,
            &member::FULL_PORTABLE,
            &member::FULL_CONTEXT_NATIVE,
            &member::FULL_DECODER_NATIVE,
            &member::FULL_DECODER_PORTABLE,
            &member::FULL_PRE_GRADE_NATIVE,
            &member::FULL_PRE_GRADE_PORTABLE,
        ],
        KernelEpoch::RowLocal => [
            &ROW_LOCAL,
            &ROW_LOCAL_V5,
            &ROW_LOCAL_V7,
            &ROW_LOCAL_V9,
            &ROW_LOCAL_V9_ALTERNATE,
            &QUALIFIED_ROW_LOCAL_V11,
            &grant4g::ROW_LOCAL_NATIVE,
            &grant4g::ROW_LOCAL_PORTABLE,
            &member::ROW_LOCAL_NATIVE,
            &member::ROW_LOCAL_RAW_VALUE_NATIVE,
            &member::ROW_LOCAL_PORTABLE,
            &member::ROW_LOCAL_CONTEXT_NATIVE,
            &member::ROW_LOCAL_DECODER_NATIVE,
            &member::ROW_LOCAL_DECODER_PORTABLE,
            &member::ROW_LOCAL_PRE_GRADE_NATIVE,
            &member::ROW_LOCAL_PRE_GRADE_PORTABLE,
        ],
    }
}

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
        let _ = executing_digest;
        if retained == executing.as_bytes() {
            return Ok(CheckedKernelReplayV1 {
                #[cfg(test)]
                digest: executing_digest,
                #[cfg(test)]
                bytes,
                semantic_contract: PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V2,
            });
        }
        if let Some(replay) = current::native(epoch, retained, configuration, bytes) {
            return Ok(replay);
        }
        let actual: [u8; 32] = Sha256::digest(retained).into();
        let original = predecessors(epoch)
            .into_iter()
            .find(|candidate| candidate.hash == actual)
            .ok_or_else(|| invalid("unsupported historical semantic kernel identity"))?;
        if !execution::compatible(configuration, original.configuration) {
            return Err(invalid(
                "historical semantic kernel compiler/configuration/dependencies differ",
            ));
        }
        Ok(CheckedKernelReplayV1 {
            #[cfg(test)]
            digest: original.digest,
            #[cfg(test)]
            bytes,
            semantic_contract: original.semantic_contract,
        })
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

#[cfg(test)]
mod tests;
