use super::*;

#[derive(Clone, Copy)]
pub(super) enum Case {
    Resolved,
    Conflicted,
    MissingOwner,
}

pub(super) struct Expected {
    pub athlete: AthleteId,
    pub current_school: SchoolId,
    pub school_name: String,
    pub case: Case,
}

impl Expected {
    pub fn status(&self) -> &'static str {
        match self.case {
            Case::Resolved => "resolved_at_selected_cutoff",
            Case::Conflicted => "conflicting_affiliation_evidence",
            Case::MissingOwner => "current_affiliation_unknown",
        }
    }
    pub fn school(&self) -> Option<&SchoolId> {
        match self.case {
            Case::Resolved => Some(&self.current_school),
            _ => None,
        }
    }
    pub fn selected_count(&self) -> usize {
        match self.case {
            Case::Resolved => 1,
            _ => 0,
        }
    }
}

pub(super) fn prepare(case: Case) -> TestResult<(fixture::Fixture, Expected)> {
    let fixture = fixture::Fixture::new()?;
    let first = fixture::install(&fixture, "12439", "42", Sport::IndoorTrack, true)?;
    fixture.accept(&first.athlete.id)?;
    let second = match case {
        Case::Resolved => None,
        Case::Conflicted | Case::MissingOwner => {
            let other = fixture::install(
                &fixture,
                "12440",
                "42",
                Sport::OutdoorTrack,
                matches!(case, Case::Conflicted),
            )?;
            crate::export::qualification::current_affiliation::tests::identity::accept_pair(
                &fixture.store,
                &fixture.writer,
                &first.athlete.id,
                &other.athlete.id,
            )?;
            Some(other)
        }
    };
    issuance::review(&fixture, &first, "controlled-publication-first-source")?;
    if let Some(second) = second {
        issuance::review(&fixture, &second, "controlled-publication-second-source")?;
    }
    let expected = Expected {
        athlete: first.athlete.id,
        current_school: first.school.id,
        school_name: first.school.name,
        case,
    };
    Ok((fixture, expected))
}
