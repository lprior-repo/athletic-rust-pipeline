pub mod collect;
pub mod map;
pub mod parse;

pub const HOST: &str = "https://chsaanow.com";
pub const SOURCE_ID: &str = "chsaa";
pub const ASSOCIATION: &str = "CHSAA";
pub const DIRECTORY_URL: &str = "https://chsaanow.com/schools/";

pub fn school_page_url(slug: &str) -> String {
    format!("{HOST}/schools/{slug}/")
}

pub(crate) use crate::row_hygiene::nonempty;

pub use collect::{collect, Options};

#[cfg(test)]
mod tests;
