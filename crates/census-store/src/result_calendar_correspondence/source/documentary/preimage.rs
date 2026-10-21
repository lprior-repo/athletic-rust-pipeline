use super::*;

pub(super) fn interpretation(
    materials: &[Material],
    body_digest: &AttachmentDigestV1,
    locator_digest: &AttachmentDigestV1,
    interpretation_digest: &AttachmentDigestV1,
    interpretation_name: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let locator = inline(materials, locator_digest, 96, grant)?;
    let body = materials
        .iter()
        .find(|material| &material.reference().digest == body_digest)
        .ok_or(Error::SourceMismatch("calendar original body reference"))?;
    locator_range(body.reference(), &locator)?;
    let interpretation = inline(materials, interpretation_digest, 256, grant)?;
    let name_bytes = interpretation_name.as_bytes();
    let expected = [
        name_bytes,
        b"\n",
        body_digest.as_str().as_bytes(),
        b"\n",
        locator.as_slice(),
    ];
    if !expected.into_iter().flatten().eq(interpretation.iter()) {
        return Err(Error::SourceMismatch("calendar interpretation preimage").into());
    }
    Ok(())
}

pub(super) fn locator_range(
    body: &AttachmentObjectReferenceV1,
    locator: &[u8],
) -> StoreResult<(u64, u64)> {
    let text = std::str::from_utf8(locator)
        .map_err(|_| Error::SourceMismatch("calendar locator UTF-8"))?;
    let (start, end) = text
        .strip_prefix("html_bytes:")
        .and_then(|text| text.split_once(':'))
        .ok_or(Error::SourceMismatch("calendar locator shape"))?;
    let start: u64 = start
        .parse()
        .map_err(|_| Error::SourceMismatch("calendar locator start"))?;
    let end: u64 = end
        .parse()
        .map_err(|_| Error::SourceMismatch("calendar locator end"))?;
    if start >= end || end > body.bytes {
        return Err(Error::SourceMismatch("calendar locator byte range").into());
    }
    Ok((start, end))
}

pub(super) fn inline(
    materials: &[Material],
    digest: &AttachmentDigestV1,
    maximum: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let material = materials
        .iter()
        .find(|material| &material.reference().digest == digest)
        .ok_or(Error::SourceMismatch("calendar required preimage absent"))?;
    if material.reference().bytes > maximum {
        return Err(Error::SourceMismatch("calendar preimage bound").into());
    }
    material.inline_bytes(grant)
}
