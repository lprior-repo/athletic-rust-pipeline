use super::*;

#[derive(Clone)]
pub(super) enum Material {
    Inline {
        reference: AttachmentObjectReferenceV1,
        bytes: Vec<u8>,
    },
    Retained(RetainedCaptureLeafV1),
}

impl Material {
    pub(super) fn reference(&self) -> &AttachmentObjectReferenceV1 {
        match self {
            Self::Inline { reference, .. } => reference,
            Self::Retained(leaf) => &leaf.object,
        }
    }

    pub(super) fn borrowed(&self) -> AttachmentContextObjectV1<'_> {
        match self {
            Self::Inline { reference, bytes } => {
                AttachmentContextObjectV1::Inline { reference, bytes }
            }
            Self::Retained(leaf) => AttachmentContextObjectV1::Retained(leaf),
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct Objects(pub(super) Vec<Material>);

impl Objects {
    pub(super) fn bytes(&mut self, bytes: Vec<u8>) -> TestResult<AttachmentDigestV1> {
        let reference = reference(&bytes)?;
        self.insert(Material::Inline { reference, bytes })
    }

    pub(super) fn json(&mut self, value: &impl Serialize) -> TestResult<AttachmentDigestV1> {
        self.bytes(serialized_bytes(value)?)
    }

    pub(super) fn insert(&mut self, material: Material) -> TestResult<AttachmentDigestV1> {
        let reference = material.reference().clone();
        if let Some(existing) = self
            .0
            .iter()
            .find(|item| item.reference().digest == reference.digest)
        {
            check!(eq; existing.reference(), &reference);
            if let (Material::Inline { bytes: left, .. }, Material::Inline { bytes: right, .. }) =
                (existing, &material)
            {
                check!(eq; left, right);
            }
        } else {
            self.0.push(material);
        }
        Ok(reference.digest)
    }

    pub(super) fn references(&self) -> Vec<AttachmentObjectReferenceV1> {
        let mut references = self
            .0
            .iter()
            .map(|item| item.reference().clone())
            .collect::<Vec<_>>();
        references.sort();
        references
    }
}

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&hash(bytes))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(super) fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

pub(super) fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}

pub(super) fn span<'a>(body: &'a [u8], start: usize, length: usize) -> TestResult<&'a [u8]> {
    let end = start.checked_add(length).ok_or("original span overflow")?;
    Ok(body.get(start..end).ok_or("exact original span absent")?)
}

pub(super) fn digest_span(
    objects: &mut Objects,
    body: &[u8],
    start: usize,
    length: usize,
    expected: &str,
) -> TestResult<AttachmentDigestV1> {
    let original = span(body, start, length)?;
    check!(eq; hash(original), expected);
    objects.bytes(original.to_vec())
}

pub(super) fn fact(
    fact: AttachmentDigestV1,
    subject: AttachmentDigestV1,
    lineage: &AttachmentDigestV1,
    disposition: AttachmentFactDispositionV1,
    rationale: &str,
) -> TestResult<ReviewedAttachmentFactV1> {
    Ok(ReviewedAttachmentFactV1 {
        fact,
        subject,
        publisher_lineage: lineage.clone(),
        disposition,
        rationale: text(rationale)?,
    })
}
