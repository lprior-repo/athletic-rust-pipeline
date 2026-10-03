const MAX_SCOPES: usize = 128;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Template,
    Svg,
    Math,
}

#[derive(Default)]
pub(super) struct Scopes {
    stack: Vec<Kind>,
    foreign: usize,
}

impl Scopes {
    pub(super) fn is_inert(&self) -> bool {
        !self.stack.is_empty()
    }

    pub(super) fn is_foreign(&self) -> bool {
        self.foreign != 0
    }

    pub(super) fn observe(
        &mut self,
        name: &[u8],
        closing: bool,
        self_closing: bool,
    ) -> Result<bool, &'static str> {
        let kind = match name {
            b"template" => Kind::Template,
            b"svg" => Kind::Svg,
            b"math" => Kind::Math,
            _ => return Ok(self.is_inert()),
        };
        if closing {
            if let Some(position) = self.stack.iter().rposition(|open| *open == kind) {
                let removed = self
                    .stack
                    .iter()
                    .skip(position)
                    .filter(|open| **open != Kind::Template)
                    .count();
                self.foreign = self
                    .foreign
                    .checked_sub(removed)
                    .ok_or("directory foreign scope accounting underflow")?;
                self.stack.truncate(position);
            }
        } else if !(self_closing && (kind != Kind::Template || self.is_foreign())) {
            if self.stack.len() >= MAX_SCOPES {
                return Err("directory HTML exceeds 128 inert container scopes");
            }
            if self.stack.capacity() == 0 {
                self.stack
                    .try_reserve_exact(MAX_SCOPES)
                    .map_err(|_| "directory scope allocation failed")?;
            }
            if kind != Kind::Template {
                self.foreign = self
                    .foreign
                    .checked_add(1)
                    .ok_or("directory foreign scope accounting overflow")?;
            }
            self.stack.push(kind);
        }
        Ok(true)
    }
}
