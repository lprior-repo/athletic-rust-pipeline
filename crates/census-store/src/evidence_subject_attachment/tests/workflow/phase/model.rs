use super::super::fixture::Context;
use super::meter::{depth, Meter};
use crate::evidence_subject_attachment::EvidenceSubjectAttachmentInspectionV1;
use crate::public_identity_tests::support::TestResult;
use census_domain::evidence_subject_attachment::*;

pub(super) struct Record {
    pub command: RetainedEvidenceSubjectAttachmentCommandV1,
    pub issued: Vec<AttachmentObjectReferenceV1>,
    pub supplied: Vec<AttachmentObjectReferenceV1>,
    pub introduction_order: Vec<AttachmentObjectReferenceV1>,
    pub introduced: Vec<AttachmentObjectReferenceV1>,
    pub inputs: [AttachmentObjectReferenceV1; 2],
    pub historical: [AttachmentObjectReferenceV1; 4],
    pub origin: usize,
}

impl Record {
    pub fn new(
        command: RetainedEvidenceSubjectAttachmentCommandV1,
        inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
        prior: &[Self],
        context: Option<&Context>,
    ) -> TestResult<Self> {
        let issued = inspection.issued_objects().cloned().collect::<Vec<_>>();
        let supplied = match context {
            Some(context) => context
                .objects
                .iter()
                .map(|item| item.reference.clone())
                .collect(),
            None => Vec::new(),
        };
        let inputs = match issued.as_slice() {
            [_, _, capture, token] | [capture, token] => [capture.clone(), token.clone()],
            _ => return Err("phase issued role count differs".into()),
        };
        let (historical, origin) = match issued.as_slice() {
            [group, witness, capture, token] => (
                [
                    group.clone(),
                    witness.clone(),
                    capture.clone(),
                    token.clone(),
                ],
                prior.len().checked_add(1).ok_or("origin overflow")?,
            ),
            [_, _] => {
                let first = prior.first().ok_or("historical first absent")?;
                (first.historical.clone(), first.origin)
            }
            _ => return Err("phase historical role count differs".into()),
        };
        let introduction_order = issued
            .iter()
            .chain(&supplied)
            .filter(|reference| {
                !prior
                    .iter()
                    .any(|record| record.introduced.contains(reference))
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut introduced = introduction_order.clone();
        introduced.sort();
        check!(eq; introduced.len(), introduction_order.len());
        check!(introduced
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left.digest < right.digest)));
        Ok(Self {
            command,
            issued,
            supplied,
            introduction_order,
            introduced,
            inputs,
            historical,
            origin,
        })
    }

    pub fn references(&self) -> &[AttachmentObjectReferenceV1] {
        self.command.reviewed_context.objects.as_slice()
    }
}

pub(super) fn find(
    records: &[&Record],
    digest: &AttachmentDigestV1,
    meter: &mut Meter,
) -> TestResult<bool> {
    for record in records {
        meter.singles(
            "chronological sorted-set lookup",
            depth(&record.introduced, digest)?,
        )?;
        if record
            .introduced
            .iter()
            .any(|reference| &reference.digest == digest)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn closure(prior: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    for reference in &record.introduced {
        meter.charge("introduction order check", 1)?;
        check!(!find(prior, &reference.digest, meter)?);
        meter.singles(
            "sorted declaration lookup",
            depth(record.references(), &reference.digest)?,
        )?;
        meter.inline()?;
    }
    for reference in record.references() {
        if !find(prior, &reference.digest, meter)? {
            meter.singles(
                "new introduction lookup",
                depth(&record.introduced, &reference.digest)?,
            )?;
        }
        meter.charge("resolved full reference equality", 1)?;
    }
    Ok(())
}

pub(super) fn introductions(prior: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    let mut novel = Vec::<&AttachmentObjectReferenceV1>::new();
    for (index, reference) in record.issued.iter().chain(&record.supplied).enumerate() {
        if index >= record.issued.len() {
            meter.charge("supplied introduction", 1)?;
        }
        if find(prior, &reference.digest, meter)? {
            check!(index < record.issued.len());
            continue;
        }
        meter.singles("inline introduction bound hash base64", 3)?;
        let less = novel
            .iter()
            .filter(|old| old.digest < reference.digest)
            .count();
        let greater = usize::from(novel.iter().any(|old| old.digest > reference.digest));
        meter.singles(
            "sorted insertion comparisons",
            less.checked_add(greater)
                .ok_or("insertion count overflow")?,
        )?;
        novel.push(reference);
    }
    check!(eq; novel.into_iter().cloned().collect::<Vec<_>>(), record.introduction_order);
    Ok(())
}
