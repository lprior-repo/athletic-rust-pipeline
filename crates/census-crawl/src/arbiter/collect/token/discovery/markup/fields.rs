use super::super::MAX_URL_BYTES;

#[derive(Default)]
pub(super) struct Name {
    bytes: [u8; 16],
    len: usize,
    overflow: bool,
}

impl Name {
    pub(super) fn clear(&mut self) {
        self.len = 0;
        self.overflow = false;
    }

    pub(super) fn push(&mut self, bytes: &[u8]) {
        let Some(end) = self.len.checked_add(bytes.len()).filter(|end| *end <= 16) else {
            self.overflow = true;
            return;
        };
        if let Some(destination) = self.bytes.get_mut(self.len..end) {
            destination.copy_from_slice(bytes);
            self.len = end;
        }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        if self.overflow {
            return &[];
        }
        self.bytes.get(..self.len).map_or(Default::default(), core::convert::identity)
    }
}

#[derive(Default)]
pub(super) struct Field {
    bytes: Vec<u8>,
    seen: bool,
    invalid: bool,
}

impl Field {
    pub(super) fn clear(&mut self) {
        self.bytes.clear();
        self.seen = false;
        self.invalid = false;
    }

    pub(super) fn push(&mut self, bytes: &[u8]) {
        if self.seen || self.invalid {
            return;
        }
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|end| end > MAX_URL_BYTES)
            || self.bytes.try_reserve(bytes.len()).is_err()
        {
            self.invalid = true;
            return;
        }
        self.bytes.extend_from_slice(bytes);
    }

    pub(super) fn finish(&mut self) {
        self.seen = true;
    }

    pub(super) fn value(&self) -> Option<&[u8]> {
        self.seen.then_some(self.bytes.as_slice())
    }

    pub(super) fn take(&mut self) -> Option<Vec<u8>> {
        self.seen.then(|| std::mem::take(&mut self.bytes))
    }

    pub(super) fn invalid(&self) -> bool {
        self.invalid
    }
}
