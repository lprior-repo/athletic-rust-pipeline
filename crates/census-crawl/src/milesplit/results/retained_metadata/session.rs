use super::{
    chunks, error, facts, manifest, run, types::ParsedMetadata, CaptureReadBudget,
    RawMetadataArchiveRef, ResultSetRef, RetainedMetadataError, VerifiedRetainedOwnedResult,
    VerifiedRetainedResultMetadata,
};
use crate::milesplit::owned::{OwnedResultSession, MAX_SESSION_CAPTURES};
use crate::milesplit::{OwnedAcquisitionRef, RetainedJournalView};
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct RetainedResultSession<'a, V> {
    view: &'a V,
    budget: CaptureReadBudget,
    owned: OwnedResultSession<'a, V>,
    metadata: BTreeMap<String, Arc<ParsedMetadata>>,
}

impl<'a, V: RetainedJournalView> RetainedResultSession<'a, V> {
    pub fn new(view: &'a V, budget: CaptureReadBudget) -> Self {
        Self {
            view,
            budget,
            owned: OwnedResultSession::new(view, budget),
            metadata: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.metadata.clear();
        self.owned.clear();
    }

    #[cfg(test)]
    pub(crate) fn resident_captures(&self) -> (usize, usize) {
        (self.owned.len(), self.metadata.len())
    }

    pub fn read(
        &mut self,
        reference: &ResultSetRef,
        archive: &RawMetadataArchiveRef,
        owned: &OwnedAcquisitionRef,
        locator: &str,
    ) -> Result<VerifiedRetainedResultMetadata, RetainedMetadataError> {
        check_reference(reference)?;
        let result = self.owned.read(owned, locator)?;
        self.bind(reference, archive, result)
    }

    pub(super) fn bind(
        &mut self,
        reference: &ResultSetRef,
        archive: &RawMetadataArchiveRef,
        result: VerifiedRetainedOwnedResult,
    ) -> Result<VerifiedRetainedResultMetadata, RetainedMetadataError> {
        check_reference(reference)?;
        if reference.meet_id.parse::<u64>().ok() != Some(result.row().meet_id)
            || reference.rsid.parse::<u64>().ok() != Some(result.row().result_set_id)
        {
            return Err(RetainedMetadataError::ForeignResult);
        }
        let parsed = self.metadata(reference, archive)?;
        Ok(VerifiedRetainedResultMetadata {
            store_identity: self.view.store_identity()?,
            snapshot_sequence: self.view.sequence(),
            division: facts::division(&result.row().provider),
            parsed,
            result,
        })
    }

    fn metadata(
        &mut self,
        reference: &ResultSetRef,
        archive: &RawMetadataArchiveRef,
    ) -> Result<Arc<ParsedMetadata>, RetainedMetadataError> {
        if let Some(parsed) = self.metadata.get(archive.manifest_key()) {
            if parsed.reference.site != reference.site
                || parsed.reference.meet_id != reference.meet_id
                || parsed.reference.rsid != reference.rsid
            {
                return Err(RetainedMetadataError::ForeignResult);
            }
            run::metadata::check_reference(&parsed.capture, reference)?;
            return Ok(Arc::clone(parsed));
        }
        if self.metadata.len() >= MAX_SESSION_CAPTURES {
            return Err(RetainedMetadataError::SessionBudget);
        }
        let parsed = Arc::new(ParsedMetadata::read(
            self.view,
            reference,
            archive,
            self.budget,
        )?);
        self.metadata
            .insert(archive.manifest_key().into(), Arc::clone(&parsed));
        Ok(parsed)
    }
}

impl ParsedMetadata {
    fn read(
        view: &impl RetainedJournalView,
        reference: &ResultSetRef,
        archive: &RawMetadataArchiveRef,
        budget: CaptureReadBudget,
    ) -> Result<Self, RetainedMetadataError> {
        let (manifest, mut capture, acquired_at) =
            manifest::load(view, reference, archive, budget)?;
        capture.body = chunks::body(view, archive, &manifest, capture.bytes)?;
        if crate::net::cache::content_digest(&capture.body) != capture.content_digest {
            return Err(error::invalid(
                "reassembled body SHA256 differs from retained digest",
            ));
        }
        let (page, start, end) = run::metadata::parse_capture_with_dates(&capture, reference)?;
        Ok(Self {
            reference: reference.clone(),
            archive: archive.clone(),
            capture,
            acquired_at,
            page,
            date: facts::date(&start, end.as_deref()),
        })
    }
}

pub(super) fn check_reference(reference: &ResultSetRef) -> Result<(), RetainedMetadataError> {
    if reference.url.len() > 4096 || reference.meet_id.len() > 20 || reference.rsid.len() > 20 {
        return Err(error::invalid(
            "result-set reference exceeds its source envelope",
        ));
    }
    Ok(())
}
