pub const MAX_RAW_KEY_BYTES: usize = 1_024;
pub const MAX_RAW_KEY_BASE64_BYTES: usize = 1_368;
pub const MAX_RAW_VALUE_BYTES: usize = 1_048_576;
pub const RAW_FRAGMENT_BYTES: usize = 524_288;
pub const MAX_PAGE_ROWS: u16 = 512;
pub const MAX_PAGE_RAW_BYTES: u32 = 16_777_216;
pub const MAX_PAGE_RECORDS: u32 = 1_537;
pub const MAX_PAGE_JSONL_BYTES: u32 = 25_165_824;
pub const MAX_RECORD_BYTES: usize = 1_048_576;
pub const MAX_REFERENCE_FANOUT: u16 = 256;
pub const MAX_DIRECTORY_RECORDS: u32 = 257;
pub const MAX_DIRECTORY_JSONL_BYTES: u32 = 131_072;
pub const MAX_SEAL_JSONL_BYTES: u32 = 65_536;
pub const MAX_TABLE_PAGES: u32 = 65_536;
pub const MAX_TABLE_ROWS: u64 = 20_000_000;
pub const MAX_GENERATION_PAGES: u32 = 393_216;
pub const MAX_GENERATION_ROWS: u64 = 120_000_000;
pub const MAX_GENERATION_RAW_BYTES: u64 = 214_748_364_800;
pub const MAX_GENERATION_OBJECTS: u32 = 394_759;
pub const MAX_GENERATION_JSONL_BYTES: u64 = 412_316_860_416;
pub const MAX_GENERATION_PHYSICAL_BYTES: u64 = 1_099_511_627_776;

#[derive(Clone, Copy)]
pub(super) struct ObjectLimits {
    pub bytes: u32,
    pub records: u32,
}

impl ObjectLimits {
    pub const PAGE: Self = Self {
        bytes: MAX_PAGE_JSONL_BYTES,
        records: MAX_PAGE_RECORDS,
    };
    pub const DIRECTORY: Self = Self {
        bytes: MAX_DIRECTORY_JSONL_BYTES,
        records: MAX_DIRECTORY_RECORDS,
    };
    pub const SEAL: Self = Self {
        bytes: MAX_SEAL_JSONL_BYTES,
        records: 1,
    };
}
