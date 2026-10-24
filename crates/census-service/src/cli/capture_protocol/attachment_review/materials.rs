use anyhow::{ensure, Context, Result};
use census_crawl::convert::read_file_capped;
use census_domain::evidence_subject_attachment::{
    check_attachment_command_with_grant, AttachmentDigestV1, AttachmentObjectReferenceV1,
    AttachmentReplayGrantV1, RetainedEvidenceSubjectAttachmentCommandV1,
};
use census_store::{AttachmentContextObjectV1, RetainedCaptureLeafV1};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(super) const COMMAND_MAX_BYTES: usize = 4 * 1024 * 1024;
const INLINE_MAX_BYTES: usize = 4 * 1024 * 1024;
const LEAF_MAX_BYTES: usize = 64 * 1024;
const SUPPLIER_MAX: usize = 64;
const SUPPLIER_MAX_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct AttachmentMaterials {
    inline: Vec<(AttachmentObjectReferenceV1, Vec<u8>)>,
    retained: Vec<RetainedCaptureLeafV1>,
}

pub(super) fn read_command(path: &Path) -> Result<RetainedEvidenceSubjectAttachmentCommandV1> {
    let bytes = read_file_capped(path, COMMAND_MAX_BYTES)
        .context("reading the bounded original attachment command")?;
    let command = serde_json::from_slice(&bytes).context("decoding the attachment command")?;
    let mut grant = AttachmentReplayGrantV1::new();
    Ok(check_attachment_command_with_grant(command, &mut grant)
        .context("checking the original attachment command")?
        .into_command())
}

fn read_supplier(path: &Path, limit: usize, remaining: &mut usize) -> Result<Vec<u8>> {
    let metadata = std::fs::metadata(path).context("reading attachment material metadata")?;
    ensure!(
        metadata.is_file(),
        "attachment material is not a regular file"
    );
    let size = usize::try_from(metadata.len()).context("attachment material length overflow")?;
    ensure!(size <= limit, "attachment material exceeds its file limit");
    ensure!(
        size <= *remaining,
        "attachment materials exceed the aggregate byte limit"
    );
    let bytes = read_file_capped(path, limit.min(*remaining))
        .context("reading the bounded attachment material")?;
    *remaining = remaining
        .checked_sub(bytes.len())
        .context("attachment material aggregate overflow")?;
    Ok(bytes)
}

fn declared(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    reference: &AttachmentObjectReferenceV1,
) -> Result<()> {
    ensure!(
        command
            .reviewed_context
            .objects
            .as_slice()
            .contains(reference),
        "attachment material reference is not declared with its exact digest and length"
    );
    Ok(())
}

pub(super) fn load(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    inline_paths: &[PathBuf],
    retained_paths: &[PathBuf],
) -> Result<AttachmentMaterials> {
    let count = inline_paths
        .len()
        .checked_add(retained_paths.len())
        .context("attachment material supplier count overflow")?;
    ensure!(
        count <= SUPPLIER_MAX,
        "attachment material supplier count exceeds 64"
    );
    let mut materials = AttachmentMaterials {
        inline: Vec::new(),
        retained: Vec::new(),
    };
    materials.inline.try_reserve_exact(inline_paths.len())?;
    materials.retained.try_reserve_exact(retained_paths.len())?;
    let mut remaining = SUPPLIER_MAX_BYTES;
    for path in inline_paths {
        let bytes = read_supplier(path, INLINE_MAX_BYTES, &mut remaining)?;
        let reference = AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(&bytes)))?,
            bytes: u64::try_from(bytes.len())?,
        };
        declared(command, &reference)?;
        materials.require_unique(&reference.digest)?;
        materials.inline.push((reference, bytes));
    }
    for path in retained_paths {
        let bytes = read_supplier(path, LEAF_MAX_BYTES, &mut remaining)?;
        let leaf: RetainedCaptureLeafV1 =
            serde_json::from_slice(&bytes).context("decoding the retained attachment leaf")?;
        ensure!(
            leaf.version == 1,
            "unsupported retained attachment leaf version"
        );
        AttachmentDigestV1::parse(&leaf.root.key)
            .context("parsing the retained attachment root key")?;
        declared(command, &leaf.object)?;
        materials.require_unique(&leaf.object.digest)?;
        materials.retained.push(leaf);
    }
    Ok(materials)
}

impl AttachmentMaterials {
    fn require_unique(&self, digest: &AttachmentDigestV1) -> Result<()> {
        ensure!(
            !self
                .inline
                .iter()
                .any(|(reference, _)| &reference.digest == digest)
                && !self
                    .retained
                    .iter()
                    .any(|leaf| &leaf.object.digest == digest),
            "duplicate attachment material supplier digest"
        );
        Ok(())
    }

    pub(super) fn borrowed(&self) -> Result<Vec<AttachmentContextObjectV1<'_>>> {
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .context("attachment material reference count overflow")?;
        let mut result = Vec::new();
        result.try_reserve_exact(count)?;
        for (reference, bytes) in &self.inline {
            result.push(AttachmentContextObjectV1::Inline { reference, bytes });
        }
        for leaf in &self.retained {
            result.push(AttachmentContextObjectV1::Retained(leaf));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
