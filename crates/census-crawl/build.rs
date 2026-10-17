#[path = "src/participation_evidence/roster_source_issuance/manifest/build_support.rs"]
mod roster_parser_manifest;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    roster_parser_manifest::generate()
}
