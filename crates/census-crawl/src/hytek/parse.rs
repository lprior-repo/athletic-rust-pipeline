use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

use super::columns::{columns_from_header, Column, Token, TEXT_LABELS};

use super::parse_rows::{heat_token, HEAT_START_SLOP, MARK_LABELS, MAX_HEAT_WIDTH};
pub(super) use super::parse_rows::{parse_legs, parse_row, starts_like_a_row};

static RELAY_LEG: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(\d+)\)\s+([^0-9]+?)\s+(\d{1,2})\b"));
static PLACE_PREFIX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d+\)"));

pub(super) fn relay_leg_regex() -> CrawlResult<&'static Regex> {
    RELAY_LEG.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "RELAY_LEG",
        source: source.clone(),
    })
}

pub(super) fn place_prefix_regex() -> CrawlResult<&'static Regex> {
    PLACE_PREFIX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PLACE_PREFIX",
            source: source.clone(),
        })
}

#[derive(Debug, Clone, Default)]
pub(super) struct Section {
    pub(in crate::hytek) columns: Vec<Column>,
    pub(super) round: Option<&'static str>,
}

impl Section {
    pub(super) fn from_header(header: &str) -> Option<Section> {
        let trimmed = header.trim_start();
        if !TEXT_LABELS.iter().any(|label| trimmed.starts_with(label)) {
            return None;
        }
        let columns = columns_from_header(header);
        if !columns
            .iter()
            .any(|column| MARK_LABELS.contains(&column.label.as_str()))
        {
            return None;
        }
        let round = columns
            .iter()
            .find_map(|column| match column.label.as_str() {
                "Prelims" | "Preliminaries" => Some("preliminaries"),
                "Semi-Finals" | "Semis" => Some("semi-finals"),
                "Finals" => Some("finals"),
                _ => None,
            });
        Some(Section { columns, round })
    }

    pub(in crate::hytek) fn column(&self, label: &str) -> Option<&Column> {
        self.columns.iter().find(|column| column.label == label)
    }

    fn numeric_token_for<'a>(&self, tokens: &[Token<'a>], column: &Column) -> Option<Token<'a>> {
        let edge = column.end.saturating_add(1);
        let previous_end = self
            .columns
            .iter()
            .filter(|other| other.start < column.start)
            .map(|other| other.end)
            .max()
            .map_or(0, |value| value);
        let trailing = !self.columns.iter().any(|other| other.start > column.start);
        tokens
            .iter()
            .filter(|token| {
                token.end <= edge
                    && (token.end.saturating_add(1) >= column.end
                        || (trailing && token.end > previous_end))
            })
            .min_by_key(|token| column.end.abs_diff(token.end))
            .copied()
    }

    pub(in crate::hytek) fn numeric_token<'a>(
        &self,
        tokens: &[Token<'a>],
        label: &str,
    ) -> Option<Token<'a>> {
        self.column(label)
            .and_then(|column| self.numeric_token_for(tokens, column))
    }

    pub(in crate::hytek) fn next_numeric_start(
        &self,
        tokens: &[Token<'_>],
        offset: usize,
    ) -> Option<usize> {
        self.columns
            .iter()
            .filter(|column| column.numeric)
            .filter_map(|column| self.numeric_token_for(tokens, column))
            .filter(|token| token.start > offset)
            .map(|token| token.start)
            .min()
    }

    pub(in crate::hytek) fn heat(&self, tokens: &[Token<'_>]) -> Option<String> {
        ["H#", "Flight", "Lane"].iter().find_map(|label| {
            let column = self.column(label)?;
            let right = self
                .columns
                .iter()
                .filter(|other| other.start > column.start)
                .map(|other| other.start)
                .min();
            tokens
                .iter()
                .filter(|token| {
                    token.start >= column.start
                        && token.start <= column.end.saturating_add(HEAT_START_SLOP)
                        && token.end <= token.start.saturating_add(MAX_HEAT_WIDTH)
                        && right.is_none_or(|edge| token.end <= edge)
                        && heat_token(token.text)
                })
                .min_by_key(|token| column.end.abs_diff(token.end))
                .map(|token| token.text.to_string())
        })
    }
    pub(in crate::hytek) fn wind(&self, tokens: &[Token<'_>]) -> Option<f64> {
        self.numeric_token(tokens, "Wind")
            .and_then(|token| token.text.parse::<f64>().ok())
            .filter(|value| value.is_finite() && value.abs() <= 12.0)
    }

    pub(in crate::hytek) fn points(&self, tokens: &[Token<'_>]) -> Option<f64> {
        ["Points", "Pts"]
            .iter()
            .find_map(|label| self.numeric_token(tokens, label))
            .and_then(|token| token.text.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
    }
}
