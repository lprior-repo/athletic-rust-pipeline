use super::super::SpecificationError;

pub(super) struct Tokens<'a> {
    values: [&'a str; 256],
    length: usize,
}

impl<'a> Tokens<'a> {
    pub(super) fn parse(label: &'a str) -> Result<Self, SpecificationError> {
        let mut tokens = Self {
            values: [""; 256],
            length: 0,
        };
        label
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '(' | ')' | '[' | ']' | ','))
            .filter(|token| !token.is_empty())
            .try_for_each(|token| tokens.push(token))?;
        Ok(tokens)
    }

    fn push(&mut self, token: &'a str) -> Result<(), SpecificationError> {
        let slot = self
            .values
            .get_mut(self.length)
            .ok_or(SpecificationError::InvalidLabel)?;
        *slot = token;
        self.length = self
            .length
            .checked_add(1)
            .ok_or(SpecificationError::InvalidLabel)?;
        Ok(())
    }

    pub(super) fn values(&self) -> Result<&[&'a str], SpecificationError> {
        self.values
            .get(..self.length)
            .ok_or(SpecificationError::InvalidLabel)
    }
}
