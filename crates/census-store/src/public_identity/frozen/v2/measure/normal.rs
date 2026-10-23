use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum ComponentBudget {
    Fits {
        remaining_bytes: usize,
    },
    Exceeded {
        component: &'static str,
        complete_component_bytes: usize,
        available_bytes: usize,
    },
}

impl ComponentBudget {
    pub(super) fn new(maximum: usize) -> Self {
        Self::Fits {
            remaining_bytes: maximum,
        }
    }
    pub(super) fn charge(&mut self, bytes: usize, component: &'static str) {
        if let Self::Fits { remaining_bytes } = *self {
            *self = match remaining_bytes.checked_sub(bytes) {
                Some(remaining_bytes) => Self::Fits { remaining_bytes },
                None => Self::Exceeded {
                    component,
                    complete_component_bytes: bytes,
                    available_bytes: remaining_bytes,
                },
            };
        }
    }
    pub(super) fn finish_object(&mut self, local: Self, bytes: usize) {
        match local {
            Self::Exceeded { .. } => *self = local,
            Self::Fits { .. } => self.charge(bytes, "object_json"),
        }
    }
}

#[cfg(test)]
mod tests;
