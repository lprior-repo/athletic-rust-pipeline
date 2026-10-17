use super::{markup, Owner};
use crate::milesplit::wire::{
    checked_span, PublishedRosterOwnerKindV1 as Kind,
    PublishedRosterOwnerTerminationV1 as Termination, PublishedRosterOwnersV1,
    PublishedRosterOwnerV1, RosterWitnessUnavailableV1 as Gap, TeamRef,
};
use crate::milesplit::RosterVerdict;
use crate::net::FetchOutcome;
use crate::CrawlResult;

pub(super) struct Published {
    pub(super) found: bool,
    pub(super) witness: Result<PublishedRosterOwnersV1, Gap>,
}

pub(super) struct Collector<'a> {
    html: &'a str,
    declarations: Result<Vec<PublishedRosterOwnerV1>, Gap>,
}

impl<'a> Collector<'a> {
    pub(super) fn new(html: &'a str) -> Self {
        Self { html, declarations: Ok(Vec::new()) }
    }

    pub(super) fn observe(&mut self, owner: Owner, kind: Kind, location: &markup::LocatedDeclaration) {
        let Err(reason) = self.push(owner, kind, location) else { return; };
        self.declarations = Err(reason);
    }

    fn push(&mut self, owner: Owner, kind: Kind, location: &markup::LocatedDeclaration) -> Result<(), Gap> {
        let declarations = self.declarations.as_mut().map_err(|reason| *reason)?;
        if declarations.len() >= 32 { return Err(Gap::OwnerInventoryLimit); }
        let span = location.span.clone().ok_or(Gap::MissingSourceSpan)?;
        let span = checked_span(self.html, span)?;
        let termination = match location.termination {
            markup::Termination::Tag => Termination::Tag,
            markup::Termination::ScriptEnd => Termination::ScriptEnd,
            markup::Termination::ScriptEof => Termination::ScriptEof,
        };
        let declaration = PublishedRosterOwnerV1::checked((owner.id, owner.jurisdiction), kind, span, termination)?;
        declarations.try_reserve(1).map_err(|_| Gap::ResourceLimit)?;
        declarations.push(declaration);
        Ok(())
    }

    pub(super) fn finish(self, found: bool) -> Published {
        Published { found, witness: self.declarations.and_then(PublishedRosterOwnersV1::checked) }
    }
}

pub(crate) fn parse_captured_roster_with_witness_v1(
    html: &str,
    team: TeamRef,
    capture: &FetchOutcome,
) -> CrawlResult<(RosterVerdict, Result<PublishedRosterOwnersV1, Gap>)> {
    super::super::body_bound(html)?;
    let published = super::validate_capture(html, &team, capture)?;
    let roster = super::super::parse_rows(html, team, super::super::StatementAuthority::Owned)?;
    Ok((roster, published.witness))
}

#[cfg(test)]
mod tests;
