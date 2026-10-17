use serde::{Deserialize, Serialize};

pub(super) const VERSION: u32 = 1;
pub(super) const POLICY: &str = "milesplit-scoped-current-roster/v1";
pub(super) const CLOSURE: &str = "roster-local-three-crate-source-closure/v1";
pub(super) const MAX_MANIFEST: usize = 64 * 1024;
pub(super) const MAX_FILES: usize = 8192;
pub(super) const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;
pub(super) const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub(super) const SOURCE_ROOTS: [&str; 3] = [
    "crates/census-crawl/src",
    "crates/census-domain/src",
    "crates/census-store/src",
];
pub(super) const CONFIG_FILES: [&str; 8] = [
    ".cargo/config.toml",
    ".cargo/fast-iteration.toml",
    "Cargo.toml",
    "crates/census-crawl/Cargo.toml",
    "crates/census-crawl/build.rs",
    "crates/census-domain/Cargo.toml",
    "crates/census-store/Cargo.toml",
    "rust-toolchain.toml",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tree {
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub files: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Schema {
    pub contract: String,
    pub source_trees: Vec<Tree>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Configuration {
    pub files: Vec<Input>,
    pub compiler: Input,
    pub environment: Vec<Input>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub version: u32,
    pub semantic_policy: String,
    pub closure_policy: String,
    pub provenance_scope: String,
    pub source_trees: Vec<Tree>,
    pub schema: Schema,
    pub configuration: Configuration,
    pub dependencies: Input,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterParserWorkingTreeV1 {
    GitReportedClean,
    GitReportedDirty,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Vcs {
    pub manifest_sha256: String,
    pub commit: String,
    pub state: RosterParserWorkingTreeV1,
}
