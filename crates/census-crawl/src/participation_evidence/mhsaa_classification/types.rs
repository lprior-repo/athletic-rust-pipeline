use super::MhsaaOriginalSpanV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MhsaaClassificationCategoryV1 {
    Boys,
    Girls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MhsaaTransitionBasisV1 {
    DirectBoysStatement,
    CoordinatedBoysStatement,
    GirlsSameSwitchesStatement,
}

#[derive(Debug)]
pub struct MhsaaClassificationTransitionV1<'a> {
    pub(super) school: MhsaaOriginalSpanV1<'a>,
    pub(super) category: MhsaaClassificationCategoryV1,
    pub(super) category_literal: MhsaaOriginalSpanV1<'a>,
    pub(super) sport_literal: MhsaaOriginalSpanV1<'a>,
    pub(super) spring_literal: MhsaaOriginalSpanV1<'a>,
    pub(super) peninsula_context: MhsaaOriginalSpanV1<'a>,
    pub(super) previous_division: MhsaaOriginalSpanV1<'a>,
    pub(super) announced_division: MhsaaOriginalSpanV1<'a>,
    pub(super) season_year: MhsaaOriginalSpanV1<'a>,
    pub(super) statement: MhsaaOriginalSpanV1<'a>,
    pub(super) basis: MhsaaTransitionBasisV1,
}

impl<'a> MhsaaClassificationTransitionV1<'a> {
    pub fn school_name(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.school
    }
    pub fn category(&self) -> MhsaaClassificationCategoryV1 {
        self.category
    }
    pub fn category_literal(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.category_literal
    }
    pub fn sport_literal(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.sport_literal
    }
    pub fn spring_literal(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.spring_literal
    }
    pub fn peninsula_context(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.peninsula_context
    }
    pub fn previous_division_literal(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.previous_division
    }
    pub fn announced_division_literal(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.announced_division
    }
    pub fn season_year_literal(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.season_year
    }
    pub fn statement(&self) -> &MhsaaOriginalSpanV1<'a> {
        &self.statement
    }
    pub fn basis(&self) -> MhsaaTransitionBasisV1 {
        self.basis
    }
}
