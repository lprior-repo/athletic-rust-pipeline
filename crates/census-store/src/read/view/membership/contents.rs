pub(crate) struct JournalNamespaceContents {
    pub(super) rows: u64,
    pub(super) encoded_bytes: u64,
    pub(super) digest: [u8; 32],
}

impl JournalNamespaceContents {
    pub(crate) fn physical_rows(&self) -> u64 {
        self.rows
    }
    pub(crate) fn encoded_bytes(&self) -> u64 {
        self.encoded_bytes
    }
    pub(crate) fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

pub(crate) struct JournalNamespaceContentsInventory {
    pub(super) contents: JournalNamespaceContents,
    pub(super) logical_roots: u64,
}

impl JournalNamespaceContentsInventory {
    pub(crate) fn contents(&self) -> &JournalNamespaceContents {
        &self.contents
    }
    pub(crate) fn logical_roots(&self) -> u64 {
        self.logical_roots
    }
}
