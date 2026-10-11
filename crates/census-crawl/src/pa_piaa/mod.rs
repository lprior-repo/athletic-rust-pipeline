pub mod collect;
pub mod map;
pub mod parse;

pub const HOST: &str = "https://www.piaa.org";
pub const SOURCE_ID: &str = "pa_piaa";
pub const ASSOCIATION: &str = "PIAA";

pub const LETTERS: [char; 24] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'Y',
];

pub const LIST_URL_PREFIX: &str = "https://www.piaa.org/schools/directory/list.aspx?alpha=";
pub const DETAILS_URL_PREFIX: &str = "https://www.piaa.org/schools/directory/details.aspx?ID=";

pub fn list_url(letter: char) -> String {
    format!("{LIST_URL_PREFIX}{letter}")
}

pub fn details_url(school_id: &str) -> String {
    format!("{DETAILS_URL_PREFIX}{school_id}")
}

pub(crate) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub use collect::{collect, Options};

#[cfg(test)]
mod tests;
