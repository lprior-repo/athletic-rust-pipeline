use super::*;

#[derive(Debug, Default)]
pub struct CheckedReviewedReferenceRecordsV1 {
    records: Vec<ReviewedReferenceRecordV1>,
    origin: Option<Arc<()>>,
}

impl CheckedReviewedReferenceRecordsV1 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> &[ReviewedReferenceRecordV1] {
        &self.records
    }

    pub fn into_records(self) -> Vec<ReviewedReferenceRecordV1> {
        self.records
    }

    pub fn push_with_grant(
        &mut self,
        record: CheckedReviewedReferenceRecordV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<()> {
        grant.admit(|grant| {
            record.require_origin(grant)?;
            self.require_origin(grant)?;
            let needed = self
                .records
                .len()
                .checked_add(1)
                .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
            limit("records", needed, ReplayLimits::SMALL.records)?;
            self.records
                .try_reserve_exact(1)
                .map_err(|_| EvidenceSubjectAttachmentError::Allocation)?;
            let (record, origin) = record.into_parts();
            if self.origin.is_none() {
                self.origin = Some(origin);
            }
            self.records.push(record);
            Ok(())
        })
    }

    pub fn pop_with_grant(
        &mut self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<CheckedReviewedReferenceRecordV1> {
        grant.admit(|grant| {
            self.require_origin(grant)?;
            if self.records.is_empty() {
                return Err(EvidenceSubjectAttachmentError::Invalid(
                    "checked record extraction absent",
                ));
            }
            let origin = grant.issue_origin()?;
            let record = self
                .records
                .pop()
                .ok_or(EvidenceSubjectAttachmentError::Invalid(
                    "checked record extraction absent",
                ))?;
            Ok(CheckedReviewedReferenceRecordV1::from_parts(record, origin))
        })
    }

    pub fn remove_with_grant(
        &mut self,
        index: usize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<CheckedReviewedReferenceRecordV1> {
        grant.admit(|grant| {
            self.require_origin(grant)?;
            if index >= self.records.len() {
                return Err(EvidenceSubjectAttachmentError::Invalid(
                    "checked record extraction absent",
                ));
            }
            let origin = grant.issue_origin()?;
            let record = self.records.remove(index);
            Ok(CheckedReviewedReferenceRecordV1::from_parts(record, origin))
        })
    }

    pub(in super::super) fn require_origin(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<()> {
        match &self.origin {
            Some(origin) => grant.require_origin(origin),
            None => grant.admit(|grant| {
                grant.steps(1)?;
                if !self.records.is_empty() {
                    return Err(EvidenceSubjectAttachmentError::Invalid(
                        "unbound checked records",
                    ));
                }
                Ok(())
            }),
        }
    }
}

impl Deref for CheckedReviewedReferenceRecordsV1 {
    type Target = [ReviewedReferenceRecordV1];

    fn deref(&self) -> &Self::Target {
        &self.records
    }
}

impl<'a> IntoIterator for &'a CheckedReviewedReferenceRecordsV1 {
    type Item = &'a ReviewedReferenceRecordV1;
    type IntoIter = std::slice::Iter<'a, ReviewedReferenceRecordV1>;

    fn into_iter(self) -> Self::IntoIter {
        self.records.iter()
    }
}
