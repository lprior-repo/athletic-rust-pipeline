use census_domain::model::{CanonicalSchool, Evidence, SourceIdentity, SourceNamespace};
use census_domain::school_directory::LinkMatch;
use census_domain::UsJurisdiction;
use census_store::StoreResult;

use super::lanes::ASSOCIATION_PREFIX;
use super::support::{
    allocation, already_owns, authority_kind, bump, bump_rule, lane_evidence, owner_id,
    parse_key_label, postal_claim, retain_evidence, stamp_website, Authority, DirectoryAuthority,
    Target,
};
use super::{Job, LaneEvidence, LaneSelection};

impl<'a> Job<'a> {
    pub(super) fn link(
        &mut self,
        school: &CanonicalSchool,
        state: UsJurisdiction,
        matched: &LinkMatch,
    ) -> StoreResult<()> {
        let target = Target {
            school,
            state,
            matched,
        };
        let Some(authority) = self.authority(target)? else {
            return Ok(());
        };
        let lane = match self.lanes.select(&authority.token, &target.matched.key) {
            Ok(lane) => lane.clone(),
            Err(LaneSelection::Missing) => {
                let detail = format!(
                    "provider {} has no lane in this generation",
                    authority.token
                );
                return self.missing_evidence(school, state, &matched.rule, detail);
            }
            Err(LaneSelection::Unattributed { candidates }) => {
                let detail = format!(
                    "no capture of {} carries {} ({}); the generation was built without \
                     per-capture provenance or lists a capture that does not hold this school, so \
                     the claim cannot name its bytes",
                    authority.token,
                    parse_key_label(&target.matched.key),
                    candidates.join(", ")
                );
                return self.missing_evidence(school, state, &matched.rule, detail);
            }
            Err(LaneSelection::Ambiguous { candidates }) => {
                let detail = format!(
                    "several captures of {} carry {} ({}); pass one capture per school or drop the \
                     duplicate before building the generation",
                    authority.token,
                    parse_key_label(&target.matched.key),
                    candidates.join(", ")
                );
                return self.refuse(school, state, &matched.rule, detail);
            }
        };
        let (Some(url), Some(observed_on)) = (lane.url.clone(), lane.observed_on.clone()) else {
            let detail = format!(
                "provider {} needs a capture URL and an observation date; pass --evidence-url {}=URL --evidence-date {}=YYYY-MM-DD, or name one capture as {provider}@{path}=… when the generation holds several",
                authority.token,
                authority.token,
                authority.token,
                provider = authority.token,
                path = lane.path
            );
            return self.missing_evidence(school, state, &matched.rule, detail);
        };
        self.commit(target, authority, lane, url, observed_on)
    }

    fn authority(&mut self, target: Target<'_>) -> StoreResult<Option<Authority>> {
        let kind = match authority_kind(&target.matched.key, &target.matched.source) {
            Some(kind) => kind,
            None => {
                let detail = format!(
                    "{} published under {} cannot own a directory identity",
                    parse_key_label(&target.matched.key),
                    target.matched.source.label()
                );
                self.refuse(target.school, target.state, &target.matched.rule, detail)?;
                return Ok(None);
            }
        };
        let (token, source, namespace) = match kind {
            DirectoryAuthority::Directory(provider) => (
                provider.to_string(),
                provider.to_string(),
                SourceNamespace::school_directory(provider, target.state),
            ),
            DirectoryAuthority::Association => match self.association_lane(target)? {
                Some(lane) => lane,
                None => return Ok(None),
            },
        };
        let Some(id) = owner_id(&target.matched.key) else {
            let detail = format!(
                "{} carries no school id",
                parse_key_label(&target.matched.key)
            );
            self.refuse(target.school, target.state, &target.matched.rule, detail)?;
            return Ok(None);
        };
        if let DirectoryAuthority::Association = kind {
            let conflict = target.school.source_identities.iter().any(|identity| {
                matches!(
                    &identity.namespace,
                    SourceNamespace::AssociationSchool { association } if association != &source
                ) && identity.id == id
            });
            if conflict {
                let detail = format!(
                    "{} is already owned by another association on this school",
                    parse_key_label(&target.matched.key)
                );
                self.refuse(target.school, target.state, &target.matched.rule, detail)?;
                return Ok(None);
            }
        }
        Ok(Some(Authority {
            token,
            source,
            namespace,
            id: id.to_string(),
        }))
    }

    fn association_lane(
        &mut self,
        target: Target<'_>,
    ) -> StoreResult<Option<(String, String, SourceNamespace)>> {
        if let Some(detail) = self.association_problem.clone() {
            self.refuse(target.school, target.state, &target.matched.rule, detail)?;
            return Ok(None);
        }
        let Some(slug) = self.association.clone() else {
            let detail = "no association lane is admitted in this generation".to_string();
            self.refuse(target.school, target.state, &target.matched.rule, detail)?;
            return Ok(None);
        };
        Ok(Some((
            format!("{ASSOCIATION_PREFIX}{slug}"),
            slug.clone(),
            SourceNamespace::association_school(&slug),
        )))
    }

    fn commit(
        &mut self,
        target: Target<'_>,
        authority: Authority,
        lane: LaneEvidence,
        url: String,
        observed_on: String,
    ) -> StoreResult<()> {
        let evidence = lane_evidence(&authority, &lane, &url, &observed_on);
        if already_owns(target.school, &authority) {
            return self.refresh(target, authority, lane, url, evidence);
        }
        self.append(target, authority, url, evidence, lane)
    }

    fn refresh(
        &mut self,
        target: Target<'_>,
        authority: Authority,
        lane: LaneEvidence,
        url: String,
        evidence: Evidence,
    ) -> StoreResult<()> {
        bump(&mut self.counters.already_linked)?;
        let mut clone = target.school.clone();
        let mut changed = stamp_website(target, &mut clone, &evidence, &mut self.counters)?;
        match postal_claim(target.matched, &authority, &lane, &url, &evidence) {
            Some(Ok(claim)) => {
                let claims = clone.postal_addresses.len();
                if let Err(error) = clone.add_postal_address(claim) {
                    return self.refuse(
                        target.school,
                        target.state,
                        &target.matched.rule,
                        error.to_string(),
                    );
                }
                changed |= clone.postal_addresses.len() > claims;
            }
            Some(Err(error)) => {
                return self.refuse(
                    target.school,
                    target.state,
                    &target.matched.rule,
                    error.to_string(),
                );
            }
            None => changed |= retain_evidence(&mut clone, &evidence)?,
        }
        if !changed {
            return Ok(());
        }
        bump(&mut self.counters.backfilled)?;
        self.push_change(clone)
    }

    fn append(
        &mut self,
        target: Target<'_>,
        authority: Authority,
        url: String,
        evidence: Evidence,
        lane: LaneEvidence,
    ) -> StoreResult<()> {
        let claim = match postal_claim(target.matched, &authority, &lane, &url, &evidence) {
            Some(Ok(claim)) => Some(claim),
            Some(Err(error)) => {
                return self.refuse(
                    target.school,
                    target.state,
                    &target.matched.rule,
                    error.to_string(),
                );
            }
            None => None,
        };
        let identity =
            SourceIdentity::new(authority.namespace.clone(), authority.id.as_str()).with_url(url);
        let mut clone = self.stamp(target, identity, &evidence, claim.is_none())?;
        let Some(claim) = claim else {
            bump(&mut self.counters.linked)?;
            bump_rule(&mut self.counters, &target.matched.rule)?;
            return self.push_change(clone);
        };
        match clone.add_postal_address(claim) {
            Ok(()) => {
                bump(&mut self.counters.linked)?;
                bump_rule(&mut self.counters, &target.matched.rule)?;
                self.push_change(clone)
            }
            Err(error) => self.refuse(
                target.school,
                target.state,
                &target.matched.rule,
                error.to_string(),
            ),
        }
    }

    fn stamp(
        &mut self,
        target: Target<'_>,
        identity: SourceIdentity,
        evidence: &Evidence,
        keep_evidence: bool,
    ) -> StoreResult<CanonicalSchool> {
        let mut clone = target.school.clone();
        stamp_website(target, &mut clone, evidence, &mut self.counters)?;
        if keep_evidence {
            clone
                .evidence
                .try_reserve(1)
                .map_err(|error| allocation(error.to_string()))?;
            clone.evidence.push(evidence.clone());
        }
        clone
            .source_identities
            .try_reserve(1)
            .map_err(|error| allocation(error.to_string()))?;
        clone.source_identities.push(identity);
        Ok(clone)
    }
}
