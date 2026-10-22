use super::*;

impl HeldGraphWitness {
    pub(super) fn rebuild_source<R: GraphRead>(
        &self,
        reader: &R,
        accepted: census_domain::model::AcceptedAthleteIdentity,
        source: census_domain::model::SourceIdentity,
        previous: &HistoryProjection,
        record: &HistoryRecord,
    ) -> StoreResult<PreparedHistoryRecord> {
        let Payload::Source {
            candidate,
            decision,
            public_id,
        } = &self.payload
        else {
            return Err(PublicIdentityError::Corrupt("graph witness payload family").into());
        };

        self.verify_decision(reader, decision, accepted.record())?;
        let digest = witness::request_digest(&record.installation, &self.operation, candidate)?;
        if digest != self.request_digest {
            return Err(PublicIdentityError::Corrupt("graph source request").into());
        }
        let evidence = CandidateDecisionReference::from_accepted(
            &accepted,
            crate::public_identity::REPLAY_LIMITS,
        )
        .map_err(PublicIdentityError::from)?;
        let rebuilt = prepare_next(
            &record.installation,
            previous.head(),
            HistoryRequest::Establish {
                binding: commit::source_binding(candidate, source, &digest),
                evidence,
            },
            crate::public_identity::REPLAY_LIMITS,
        )
        .map_err(PublicIdentityError::from)?;
        if rebuilt
            .established_ids
            .iter()
            .map(PublicAthleteId::as_str)
            .ne(std::iter::once(public_id.as_str()))
        {
            return Err(PublicIdentityError::Corrupt("graph source public ID").into());
        }
        Ok(rebuilt)
    }
    pub(super) fn rebuild_reviewed<R: GraphRead>(
        &self,
        reader: &R,
        view: &VerifiedRawGraph<'_, R>,
        previous: &HistoryProjection,
        record: &HistoryRecord,
    ) -> StoreResult<PreparedHistoryRecord> {
        let Payload::Reviewed {
            kind,
            case_id,
            decision,
            introduced_public_ids,
        } = &self.payload
        else {
            return Err(PublicIdentityError::Corrupt("graph witness payload family").into());
        };

        let request =
            correction::ReviewedIdentityRequestV1::from_history(case_id, &record.request)?;
        if request.kind() != *kind
            || self.request_digest
                != correction::request::digest(&record.installation, &self.operation, &request)?
        {
            return Err(PublicIdentityError::Corrupt("graph reviewed request").into());
        }
        let accepted = graph::accept_reviewed(view, case_id, &self.observed_at)?;
        correction::affected::validate(view, previous, &request)?;
        self.verify_decision(reader, decision, accepted.record())?;
        let rebuilt = prepare_next(
            &record.installation,
            previous.head(),
            request.authorized(&accepted)?,
            crate::public_identity::REPLAY_LIMITS,
        )
        .map_err(PublicIdentityError::from)?;
        if rebuilt
            .established_ids
            .iter()
            .map(PublicAthleteId::as_str)
            .ne(introduced_public_ids.iter().map(String::as_str))
        {
            return Err(PublicIdentityError::Corrupt("graph reviewed public IDs").into());
        }
        Ok(rebuilt)
    }
    pub(super) fn rebuild_membership<R: GraphRead>(
        &self,
        reader: &R,
        view: &VerifiedRawGraph<'_, R>,
        previous: &HistoryProjection,
        record: &HistoryRecord,
    ) -> StoreResult<PreparedHistoryRecord> {
        let Payload::Membership { request, decision } = &self.payload else {
            return Err(PublicIdentityError::Corrupt("graph witness payload family").into());
        };

        let digest = membership::request::digest(&record.installation, &self.operation, request)?;
        if digest != self.request_digest {
            return Err(PublicIdentityError::Corrupt("graph membership request").into());
        }
        let accepted = graph::accept_reviewed(view, &request.case_id, &self.observed_at)?;
        self.verify_decision(reader, decision, accepted.record())?;
        let anchor = previous
            .binding_claim(&request.anchor_binding_id)
            .ok_or(PublicIdentityError::NotSourceBound)?;
        let rows =
            graph::member_rows(view, &[anchor.candidate.clone(), request.candidate.clone()])?;
        let authorized =
            membership::request::authorized_rows(request, &digest, &rows, &accepted, previous)?;
        prepare_next(
            &record.installation,
            previous.head(),
            authorized,
            crate::public_identity::REPLAY_LIMITS,
        )
        .map_err(PublicIdentityError::from)
        .map_err(Into::into)
    }
}
