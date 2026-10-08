const MAX_DEPTH: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Container {
    Template,
    Svg,
    Math,
}

impl Container {
    pub(super) fn from_tag(tag: &[u8]) -> Option<Self> {
        match tag {
            b"template" => Some(Self::Template),
            b"svg" => Some(Self::Svg),
            b"math" => Some(Self::Math),
            _ => None,
        }
    }
}

pub(super) struct Containers {
    entries: [Option<Container>; MAX_DEPTH],
    depth: usize,
}

impl Default for Containers {
    fn default() -> Self {
        Self {
            entries: [None; MAX_DEPTH],
            depth: 0,
        }
    }
}

impl Containers {
    pub(super) fn is_empty(&self) -> bool {
        self.depth == 0
    }

    pub(super) fn open(&mut self, container: Container) -> bool {
        let Some(next) = self.depth.checked_add(1) else {
            return false;
        };
        let Some(entry) = self.entries.get_mut(self.depth) else {
            return false;
        };
        *entry = Some(container);
        self.depth = next;
        true
    }

    pub(super) fn close(&mut self, container: Container) -> bool {
        let Some(previous) = self.depth.checked_sub(1) else {
            return false;
        };
        let Some(entry) = self.entries.get_mut(previous) else {
            return false;
        };
        if *entry != Some(container) {
            return false;
        }
        *entry = None;
        self.depth = previous;
        true
    }
}
