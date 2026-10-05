pub const SOURCE_ID: &str = "wikidata";

pub const SPARQL_ENDPOINT: &str = "https://query.wikidata.org/sparql";

mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
pub use parse::{parse_sparql, sparql_query};
