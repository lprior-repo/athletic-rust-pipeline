use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRowLocator {
    #[serde(rename = "ordinal")]
    ordinal: u32,

    #[serde(rename = "byte_offset")]
    byte_offset: usize,

    #[serde(rename = "byte_length")]
    byte_length: usize,
}

impl SourceRowLocator {
    pub fn new(ordinal: u32, byte_offset: usize, byte_length: usize) -> Self {
        Self {
            ordinal,
            byte_offset,
            byte_length,
        }
    }

    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub fn byte_length(&self) -> usize {
        self.byte_length
    }
}
