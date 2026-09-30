pub const SOURCE_ID: &str = "private_assoc";

mod parse;

#[cfg(test)]
mod tests;

pub use parse::parse_listing;
