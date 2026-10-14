use super::*;

impl CoachLaneMatrix {
    pub fn new(rows: Vec<CoachLaneObligation>) -> Self {
        Self { rows }
    }

    pub fn rows(&self) -> &[CoachLaneObligation] {
        self.rows.as_slice()
    }

    pub fn verify(&self) -> Result<(), CoachLaneError> {
        for &jurisdiction in UsJurisdiction::CENSUS_SCOPE.iter() {
            let mut found: Option<&CoachLaneObligation> = None;
            let mut duplicated = false;
            for row in self
                .rows
                .iter()
                .filter(|row| row.jurisdiction() == jurisdiction)
            {
                if found.is_some() {
                    duplicated = true;
                }
                found = Some(row);
            }
            let Some(row) = found else {
                return Err(CoachLaneError::MissingJurisdiction { jurisdiction });
            };
            if duplicated {
                return Err(CoachLaneError::DuplicateJurisdiction { jurisdiction });
            }
            row.validate()?;
            if !row.is_terminal() {
                return Err(CoachLaneError::LaneNotClosed {
                    jurisdiction,
                    status: row.status(),
                });
            }
        }
        self.rows.iter().try_for_each(|row| {
            if row.jurisdiction().is_in_census_scope() {
                Ok(())
            } else {
                Err(CoachLaneError::OutsideCensusScope {
                    jurisdiction: row.jurisdiction(),
                })
            }
        })
    }
}
