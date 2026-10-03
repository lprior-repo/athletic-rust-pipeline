#[derive(Default)]
pub(super) struct Name {
    bytes: Vec<u8>,
    overflow: bool,
}

impl Name {
    pub(super) fn clear(&mut self) {
        self.bytes.clear();
        self.overflow = false;
    }

    pub(super) fn push(&mut self, bytes: &[u8]) {
        if self.overflow {
            return;
        }
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|size| size > 16)
        {
            self.bytes.clear();
            self.overflow = true;
        } else {
            self.bytes.extend_from_slice(bytes);
        }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Copy)]
pub(super) enum Attribute {
    Rel,
    Href,
    Property,
    Name,
    Content,
    Type,
}

impl Attribute {
    fn index(self) -> usize {
        match self {
            Self::Rel => 0,
            Self::Href => 1,
            Self::Property => 2,
            Self::Name => 3,
            Self::Content => 4,
            Self::Type => 5,
        }
    }

    fn bit(self) -> u8 {
        match self {
            Self::Rel => 1,
            Self::Href => 2,
            Self::Property => 4,
            Self::Name => 8,
            Self::Content => 16,
            Self::Type => 32,
        }
    }

    pub(super) fn parse(name: &[u8]) -> Option<Self> {
        match name {
            b"rel" => Some(Self::Rel),
            b"href" => Some(Self::Href),
            b"property" => Some(Self::Property),
            b"name" => Some(Self::Name),
            b"content" => Some(Self::Content),
            b"type" => Some(Self::Type),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct Fields {
    values: [Vec<u8>; 6],
    seen: u8,
}

impl Fields {
    pub(super) fn clear(&mut self) {
        self.values.iter_mut().for_each(Vec::clear);
        self.seen = 0;
    }

    pub(super) fn push(&mut self, attribute: Attribute, bytes: &[u8]) {
        if self.seen & attribute.bit() == 0 {
            if let Some(value) = self.values.get_mut(attribute.index()) {
                value.extend_from_slice(bytes);
            }
        }
    }

    pub(super) fn finish(&mut self, attribute: Attribute) {
        self.seen |= attribute.bit();
    }

    pub(super) fn value(&self, attribute: Attribute) -> Option<&[u8]> {
        (self.seen & attribute.bit() != 0)
            .then(|| self.values.get(attribute.index()).map(Vec::as_slice))
            .flatten()
    }

    pub(super) fn take(&mut self, attribute: Attribute) -> Option<Vec<u8>> {
        (self.seen & attribute.bit() != 0)
            .then(|| self.values.get_mut(attribute.index()).map(std::mem::take))
            .flatten()
    }
}
