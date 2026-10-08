mod parse;
mod regexes;
mod table;
mod title;

pub use parse::parse;

#[cfg(test)]
use title::division_of;

#[cfg(test)]
use census_domain::model::{EventKind, ExactSeconds, Gender, Grade, Mark, SourceRef};

#[cfg(test)]
mod tests;
