use super::patterns::{
    ad_patterns, coach_patterns, AD_PATTERNS, ANCHOR, ASSISTANT, ATTRIBUTE, BOYS, CELL,
    COACH_PATTERNS, COMMENT, CROSS_COUNTRY, DOMAIN, GIRLS, HIDDEN_BLOCK, INTERESTING, JUNK_CONTEXT,
    LOCAL, OFF_LIMITS, PLATFORM, ROW, SPORT_HINT, TABLE, TABLE_HEADER, TAG, TITLE_TAG, TOKEN,
};
use super::urls::{base_domain, host_of};
use crate::directory::compile_pattern;
use crate::{CrawlError, CrawlResult};
use regex::Regex;

fn compile(pattern: &str, name: &'static str) -> CrawlResult<Regex> {
    Regex::new(pattern).map_err(|source| CrawlError::RegexInit {
        pattern: name,
        source,
    })
}

fn compile_all(patterns: &[String], names: &[&'static str]) -> CrawlResult<Vec<Regex>> {
    patterns
        .iter()
        .zip(names.iter())
        .map(|(pattern, name)| compile(pattern, name))
        .collect()
}

pub struct Rules {
    pub(super) mailto: Regex,
    pub(super) bare_mail: Regex,
    pub(super) token: Regex,
    pub(super) inline_gap: Regex,
    pub(super) any_gap: Regex,
    pub(super) coach: Vec<Regex>,
    pub(super) ad: Vec<Regex>,
    pub(super) sport_hint: Regex,
    pub(super) interesting: Regex,
    pub(super) off_limits: Regex,
    pub(super) platform: Regex,
    pub(super) junk: Regex,
    pub(super) table_header: Regex,
    pub(super) hidden_block: Regex,
    pub(super) comment: Regex,
    pub(super) tag: Regex,
    pub(super) title_tag: Regex,
    pub(super) anchor: Regex,
    pub(super) attribute: Regex,
    pub(super) table: Regex,
    pub(super) row: Regex,
    pub(super) cell: Regex,
    pub(super) local: Regex,
    pub(super) domain: Regex,
    pub(super) girls: Regex,
    pub(super) boys: Regex,
    pub(super) cross_country: Regex,
    pub(super) assistant: Regex,
}

impl Rules {
    pub fn new() -> CrawlResult<Self> {
        let coach = compile_all(&coach_patterns(), &COACH_PATTERNS)?;
        let ad = compile_all(&ad_patterns(), &AD_PATTERNS)?;
        Ok(Self {
            mailto: compile_pattern(r#"(?i)mailto:([^"'?>\s]+)"#, "school_sites mailto")?,
            bare_mail: compile_pattern(
                r"\b[\w.+-]+@[\w-]+\.[\w.-]{2,}\b",
                "school_sites bare mail",
            )?,
            token: compile_pattern(TOKEN, "school_sites token")?,
            inline_gap: compile_pattern(r"[ \t\u{a0}]+", "school_sites inline gap")?,
            any_gap: compile_pattern(r"\s+", "school_sites any gap")?,
            coach,
            ad,
            sport_hint: compile_pattern(SPORT_HINT, "school_sites sport hint")?,
            interesting: compile_pattern(INTERESTING, "school_sites interesting")?,
            off_limits: compile_pattern(OFF_LIMITS, "school_sites off limits")?,
            platform: compile_pattern(PLATFORM, "school_sites platform")?,
            junk: compile_pattern(JUNK_CONTEXT, "school_sites junk context")?,
            table_header: compile_pattern(TABLE_HEADER, "school_sites table header")?,
            hidden_block: compile_pattern(HIDDEN_BLOCK, "school_sites hidden block")?,
            comment: compile_pattern(COMMENT, "school_sites comment")?,
            tag: compile_pattern(TAG, "school_sites tag")?,
            title_tag: compile_pattern(TITLE_TAG, "school_sites title")?,
            anchor: compile_pattern(ANCHOR, "school_sites anchor")?,
            attribute: compile_pattern(ATTRIBUTE, "school_sites attribute")?,
            table: compile_pattern(TABLE, "school_sites table")?,
            row: compile_pattern(ROW, "school_sites row")?,
            cell: compile_pattern(CELL, "school_sites cell")?,
            local: compile_pattern(LOCAL, "school_sites local")?,
            domain: compile_pattern(DOMAIN, "school_sites domain")?,
            girls: compile_pattern(GIRLS, "school_sites girls")?,
            boys: compile_pattern(BOYS, "school_sites boys")?,
            cross_country: compile_pattern(CROSS_COUNTRY, "school_sites cross country")?,
            assistant: compile_pattern(ASSISTANT, "school_sites assistant")?,
        })
    }

    pub fn interesting(&self, href: &str, label: &str) -> bool {
        self.interesting.is_match(href) || self.interesting.is_match(label)
    }

    pub fn followable(&self, href: &str, site: &str) -> bool {
        if !href.starts_with("http") {
            return true;
        }
        let Some(host) = host_of(href) else {
            return false;
        };
        let host = host.to_ascii_lowercase();
        if self.off_limits.is_match(&host) {
            return false;
        }
        let Some(site_host) = host_of(site) else {
            return false;
        };
        let site_host = site_host.to_ascii_lowercase();
        if host.contains(&site_host) || self.platform.is_match(&host) {
            return true;
        }
        base_domain(&host) == base_domain(&site_host)
    }
}
