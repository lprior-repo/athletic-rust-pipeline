use crate::{
    config::OllamaConfig,
    marks,
    model::{Candidate, Mark, ModelDecision, Prospect, SearchHit},
};
use anyhow::{Context, Result};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::LazyLock, time::Duration};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
    api: String,
    enabled: bool,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaMessage {
    content: String,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAiResponse {
    #[serde(default)]
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAiMessage {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct CandidateExtraction {
    #[serde(default)]
    athlete_name: String,
    #[serde(default)]
    school: String,
    #[serde(default)]
    location: String,
    graduation_year: Option<i32>,
    #[serde(default)]
    sports: Vec<String>,
    #[serde(default)]
    marks: Vec<ExtractedMark>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ExtractedMark {
    #[serde(default)]
    event: String,
    #[serde(default)]
    mark: String,
    #[serde(default)]
    season: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    meet_name: String,
    wind: Option<String>,
    #[serde(default)]
    is_pr: bool,
}

#[derive(Debug, Serialize)]
struct CandidateSummary<'a> {
    index: usize,
    profile_url: &'a str,
    athlete_name: &'a str,
    school: &'a str,
    location: &'a str,
    graduation_year: Option<i32>,
    sports: &'a [String],
    deterministic_score: f64,
    corroborated: bool,
}

impl OllamaClient {
    pub fn new(config: &OllamaConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;
        Ok(Self {
            client,
            base_url: config.url.trim_end_matches('/').to_owned(),
            model: config.model.clone(),
            api: config.api.clone(),
            enabled: config.enabled,
        })
    }

    async fn chat_json<T: for<'de> Deserialize<'de>>(&self, prompt: &str) -> Result<T> {
        if !self.enabled {
            anyhow::bail!("Local model is disabled");
        }
        let openai_compatible = self.api == "openai-compatible";
        let endpoint = if openai_compatible {
            format!("{}/v1/chat/completions", self.base_url)
        } else {
            format!("{}/api/chat", self.base_url)
        };
        let messages = json!([
            {
                "role": "system",
                "content": "Return JSON only. Never invent identity evidence, marks, dates, schools, or class years. Missing evidence must remain empty or null."
            },
            { "role": "user", "content": prompt }
        ]);
        let body = if openai_compatible {
            json!({
                "model": self.model,
                "messages": messages,
                "temperature": 0.0,
                "max_tokens": 512,
                "reasoning_effort": "none",
                "chat_template_kwargs": { "enable_thinking": false },
                "response_format": { "type": "json_object" },
            })
        } else {
            json!({
                "model": self.model,
                "stream": false,
                "think": false,
                "format": "json",
                "options": {
                    "temperature": 0.0,
                    "num_predict": 512,
                },
                "messages": messages
            })
        };
        let response = self
            .client
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("calling local model at {endpoint}"))?
            .error_for_status()
            .context("local model returned an error status")?;
        let content = if openai_compatible {
            response
                .json::<OpenAiResponse>()
                .await
                .context("decoding OpenAI-compatible local model response")?
                .choices
                .into_iter()
                .next()
                .map(|choice| choice.message.content)
                .filter(|value| !value.trim().is_empty())
                .context("local model returned no text choice")?
        } else {
            response
                .json::<OllamaResponse>()
                .await
                .context("decoding Ollama response")?
                .message
                .content
        };
        let object = extract_json_object(&content)
            .context("local model response did not contain a JSON object")?;
        serde_json::from_str(object).context("decoding structured local model JSON")
    }

    async fn extract_candidate(
        &self,
        prospect: &Prospect,
        hit: &SearchHit,
        evidence: &str,
    ) -> Result<CandidateExtraction> {
        let prompt = format!(
            r#"Extract only facts explicitly supported by the candidate evidence.

Prospect context is supplied only to focus extraction, not as evidence:
name: {prospect_name}
school: {prospect_school}
location: {prospect_city}, {prospect_state}
expected graduation year: {year:?}
requested sport: {sport}

Candidate URL: {url}
Search title: {title}
Search snippet: {snippet}
Candidate page/search evidence:
{evidence}

Return exactly this JSON shape:
{{
  "athlete_name": "",
  "school": "",
  "location": "",
  "graduation_year": null,
  "sports": [],
  "marks": [
    {{
      "event": "",
      "mark": "",
      "season": "",
      "date": "",
      "meet_name": "",
      "wind": null,
      "is_pr": false
    }}
  ]
}}

Do not copy prospect fields into the candidate unless the evidence independently shows them.
Do not infer a PR when the evidence does not label it or provide enough complete results to establish it."#,
            prospect_name = prospect.full_name(),
            prospect_school = prospect.school,
            prospect_city = prospect.city,
            prospect_state = prospect.state,
            year = prospect.expected_graduation_year,
            sport = prospect.sport,
            url = hit.url,
            title = hit.title,
            snippet = hit.snippet,
        );
        self.chat_json(&prompt).await
    }

    pub async fn validate_identity(
        &self,
        prospect: &Prospect,
        candidates: &[Candidate],
    ) -> ModelDecision {
        if !self.enabled {
            return ModelDecision {
                decision: "DETERMINISTIC".to_owned(),
                model_status: "disabled".to_owned(),
                reason: "Ollama validation disabled".to_owned(),
                ..Default::default()
            };
        }
        let summaries: Vec<CandidateSummary<'_>> = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| CandidateSummary {
                index,
                profile_url: &candidate.profile_url,
                athlete_name: &candidate.athlete_name,
                school: &candidate.school,
                location: &candidate.location,
                graduation_year: candidate.graduation_year,
                sports: &candidate.sports,
                deterministic_score: candidate.deterministic_score,
                corroborated: candidate.corroborated,
            })
            .collect();
        let prospect_json = match serde_json::to_string_pretty(prospect) {
            Ok(value) => value,
            Err(error) => {
                return ModelDecision {
                    decision: "DETERMINISTIC".to_owned(),
                    model_status: "serialization_error".to_owned(),
                    reason: format!("Could not serialize prospect context: {error}"),
                    ..Default::default()
                };
            }
        };
        let summaries_json = match serde_json::to_string_pretty(&summaries) {
            Ok(value) => value,
            Err(error) => {
                return ModelDecision {
                    decision: "DETERMINISTIC".to_owned(),
                    model_status: "serialization_error".to_owned(),
                    reason: format!("Could not serialize candidate context: {error}"),
                    ..Default::default()
                };
            }
        };
        let prompt = format!(
            r#"Perform conservative identity review. Exact name alone is insufficient. School, geography, class year, and team/sport must corroborate identity. Cross Country may corroborate Track & Field but is not a substitute for Track participation. Conflicting school/state/year is negative evidence. Do not invent facts.

Prospect:
{prospect_json}

Candidates (zero-based index):
{summaries_json}

Return exactly:
{{
  "decision": "MATCH|CLOSE_MATCH|REVIEW|NO_MATCH",
  "candidate_index": null,
  "confidence": 0.0,
  "track_confirmed": false,
  "xc_confirmed": false,
  "reason": "",
  "model_status": "ok"
}}

Use candidate_index only when one candidate is defensible. False positives are worse than false negatives."#
        );
        match self.chat_json::<ModelDecision>(&prompt).await {
            Ok(mut decision) => {
                decision.model_status = "ok".to_owned();
                if decision
                    .candidate_index
                    .is_some_and(|index| index >= candidates.len())
                {
                    decision.candidate_index = None;
                    decision.decision = "REVIEW".to_owned();
                    decision
                        .reason
                        .push_str(" Invalid candidate index returned by model.");
                    decision.model_status = "invalid_index".to_owned();
                }
                decision.confidence = decision.confidence.clamp(0.0, 1.0);
                decision
            }
            Err(error) => ModelDecision {
                decision: "DETERMINISTIC".to_owned(),
                model_status: "unavailable_or_invalid".to_owned(),
                reason: format!("Local model validation unavailable: {error:#}"),
                ..Default::default()
            },
        }
    }
}

pub async fn candidate_from_evidence(
    prospect: &Prospect,
    hit: &SearchHit,
    html: Option<&str>,
    ollama: &OllamaClient,
    page_text_limit: usize,
) -> Candidate {
    let evidence = build_evidence(hit, html, page_text_limit);
    let mut extraction = match ollama.extract_candidate(prospect, hit, &evidence).await {
        Ok(value) => value,
        Err(_) => fallback_extraction(hit),
    };
    enrich_from_search_evidence(prospect, hit, &mut extraction);
    let marks = extraction
        .marks
        .into_iter()
        .map(|item| {
            marks::normalize_mark(Mark {
                event: item.event,
                mark: item.mark,
                season: item.season,
                date: item.date,
                meet_name: item.meet_name,
                wind: item.wind,
                source_url: hit.url.clone(),
                is_pr_claimed: item.is_pr,
                ..Default::default()
            })
        })
        .collect();
    Candidate {
        profile_url: hit.url.clone(),
        search_title: hit.title.clone(),
        search_snippet: hit.snippet.clone(),
        athlete_name: extraction.athlete_name,
        school: extraction.school,
        location: extraction.location,
        graduation_year: extraction.graduation_year,
        sports: extraction.sports,
        marks,
        page_retrieved: html.is_some(),
        evidence_text: evidence.chars().take(4000).collect(),
        evidence_urls: vec![hit.url.clone()],
        ..Default::default()
    }
}

static MARK_PAIR: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r#"(?ix)\b(
            100mH|110mH|300mH|400mH|55m|60m|100m|200m|300m|400m|600m|800m|
            1000m|1500m|1600m|mile|3000m|3200m|5000m|
            high\s+jump|long\s+jump|triple\s+jump|pole\s+vault|shot(?:\s+put)?|
            discus|javelin|HJ|LJ|TJ|PV|SP|DT|JAV
        )\s+(
            \d+\s*(?:'|-)\s*\d+(?:\.\d+)?(?:["a-z])?|
            (?:\d+:)?\d+(?:\.\d+)?[a-z]?
        )"#,
    )
    .ok()
});

fn enrich_from_search_evidence(
    prospect: &Prospect,
    hit: &SearchHit,
    extraction: &mut CandidateExtraction,
) {
    let search_text = format!("{} {}", hit.title, hit.snippet);
    let location = matching_location(prospect, &search_text);
    if extraction.athlete_name.trim().is_empty()
        || extraction.athlete_name.split_whitespace().count() > 5
    {
        if let Some(location) = location.as_deref() {
            if let Some(name) = name_before_location(&hit.title, location) {
                extraction.athlete_name = name;
            }
        }
    }
    if extraction.location.trim().is_empty() {
        if let Some(location) = location {
            extraction.location = location;
        }
    }
    if extraction.school.trim().is_empty() {
        if let Some(school) = matching_school(prospect, &search_text) {
            extraction.school = school;
        }
    }
    if extraction.marks.is_empty() {
        extraction.marks = search_marks(&hit.snippet);
    }
    if extraction.sports.is_empty() {
        extraction.sports = if hit.url.contains("cross-country") {
            vec!["Cross Country".to_owned()]
        } else {
            vec!["Track & Field".to_owned()]
        };
    }
}

fn matching_location(prospect: &Prospect, text: &str) -> Option<String> {
    let city = prospect.city.trim();
    let state = prospect.state.trim();
    if city.is_empty() || state.is_empty() {
        return None;
    }
    let expected = normalize_text(&format!("{city} {state}"));
    let observed = normalize_text(text);
    observed
        .contains(&expected)
        .then(|| format!("{city}, {state}"))
}

fn name_before_location(title: &str, location: &str) -> Option<String> {
    let title_lower = title.to_ascii_lowercase();
    let location_lower = location.to_ascii_lowercase();
    let end = title_lower.find(&location_lower)?;
    let name = title.get(..end)?.trim().trim_end_matches([',', '-', ' ']);
    let name = name.split("...").next()?.trim();
    let token_count = name.split_whitespace().count();
    (name.len() >= 2 && token_count <= 5).then(|| name.to_owned())
}

fn matching_school(prospect: &Prospect, text: &str) -> Option<String> {
    let school = prospect.school.trim();
    if school.is_empty() {
        return None;
    }
    let normalized_text = normalize_text(text);
    let normalized_school = normalize_text(school);
    let short_school = normalized_school
        .strip_suffix(" high school")
        .or_else(|| normalized_school.strip_suffix(" hs"))
        .map_or_else(|| normalized_school.clone(), ToOwned::to_owned);
    let aliases = [
        normalized_school,
        format!("{short_school} high school"),
        format!("{short_school} hs"),
    ];
    aliases
        .iter()
        .any(|alias| alias.len() >= 5 && normalized_text.contains(alias))
        .then(|| school.to_owned())
}

fn search_marks(text: &str) -> Vec<ExtractedMark> {
    let Some(pattern) = MARK_PAIR.as_ref() else {
        return Vec::new();
    };
    pattern
        .captures_iter(text)
        .filter_map(|capture| {
            let event = capture
                .get(1)?
                .as_str()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let raw_mark = capture.get(2)?.as_str().trim();
            let wind = raw_mark
                .chars()
                .last()
                .filter(|character| matches!(character.to_ascii_lowercase(), 'w' | 'a'))
                .map(|character| character.to_string());
            let mark = wind.as_deref().map_or_else(
                || raw_mark.to_owned(),
                |suffix| raw_mark.trim_end_matches(suffix).trim().to_owned(),
            );
            Some(ExtractedMark {
                event: canonical_search_event(&event),
                mark,
                wind,
                ..Default::default()
            })
        })
        .collect()
}

fn canonical_search_event(event: &str) -> String {
    match normalize_text(event).as_str() {
        "hj" | "high jump" => "high jump".to_owned(),
        "lj" | "long jump" => "long jump".to_owned(),
        "tj" | "triple jump" => "triple jump".to_owned(),
        "pv" | "pole vault" => "pole vault".to_owned(),
        "sp" | "shot" | "shot put" => "shot put".to_owned(),
        "dt" | "discus" => "discus".to_owned(),
        "jav" | "javelin" => "javelin".to_owned(),
        _ => event.to_owned(),
    }
}

fn normalize_text(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fallback_extraction(hit: &SearchHit) -> CandidateExtraction {
    let first_segment = match hit.title.split(" - ").next() {
        Some(value) => value,
        None => hit.title.as_str(),
    };
    let title = match first_segment.split(" | ").next() {
        Some(value) => value,
        None => hit.title.as_str(),
    }
    .trim()
    .to_owned();
    CandidateExtraction {
        athlete_name: title,
        sports: if hit.url.contains("track-and-field") {
            vec!["Track & Field".to_owned()]
        } else if hit.url.contains("cross-country") {
            vec!["Cross Country".to_owned()]
        } else {
            Vec::new()
        },
        ..Default::default()
    }
}

fn build_evidence(hit: &SearchHit, html: Option<&str>, limit: usize) -> String {
    let mut output = format!(
        "Search title: {}\nSearch snippet: {}\n",
        hit.title, hit.snippet
    );
    if let Some(html) = html {
        let compact = compact_html(html);
        output.push_str("Page text/data:\n");
        output.extend(compact.chars().take(limit.saturating_sub(output.len())));
    }
    output
}

static HTML_SCRIPT_STYLE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?is)<script\b[^>]*>.*?</script>|<style\b[^>]*>.*?</style>").ok()
});
static HTML_TAGS: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").ok());
static HTML_SPACE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\s+").ok());

fn compact_html(html: &str) -> String {
    let without_scripts = HTML_SCRIPT_STYLE.as_ref().map_or_else(
        || html.to_owned(),
        |regex| regex.replace_all(html, " ").into_owned(),
    );
    let without_tags = HTML_TAGS.as_ref().map_or_else(
        || without_scripts.clone(),
        |regex| regex.replace_all(&without_scripts, " ").into_owned(),
    );
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    HTML_SPACE.as_ref().map_or_else(
        || decoded.trim().to_owned(),
        |regex| regex.replace_all(&decoded, " ").trim().to_owned(),
    )
}

fn extract_json_object(value: &str) -> Option<&str> {
    let start = value.find('{')?;
    let end = value.rfind('}')?;
    if end < start {
        return None;
    }
    value.get(start..=end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_wrapped_json() {
        assert_eq!(
            extract_json_object("```json\n{\"a\":1}\n```"),
            Some("{\"a\":1}")
        );
    }

    #[test]
    fn compacts_html_text() {
        assert_eq!(compact_html("<h1>A &amp; B</h1>\n<p>C</p>"), "A & B C");
    }

    #[test]
    fn removes_scripts_before_model_evidence() {
        assert_eq!(
            compact_html("<script>secret noise</script><p>visible marks</p><style>.x{}</style>"),
            "visible marks"
        );
    }

    #[test]
    fn enriches_search_identity_and_performances() {
        let prospect = Prospect {
            first_name: "Taylor".to_owned(),
            last_name: "Example".to_owned(),
            school: "Example High School".to_owned(),
            city: "Testville".to_owned(),
            state: "TS".to_owned(),
            sport: "Track and Field: Womens".to_owned(),
            ..Default::default()
        };
        let hit = SearchHit {
            url: "https://www.athletic.net/athlete/99999999/track-and-field".to_owned(),
            title: "Taylor Example Testville, TS Example Track Club".to_owned(),
            snippet: "Taylor Example Testville, TS Example HS (2024-2026) 100mH 18.65a HJ 1.65m"
                .to_owned(),
            ..Default::default()
        };
        let mut extraction = CandidateExtraction::default();
        enrich_from_search_evidence(&prospect, &hit, &mut extraction);
        assert_eq!(extraction.athlete_name, "Taylor Example");
        assert_eq!(extraction.school, "Example High School");
        assert_eq!(extraction.location, "Testville, TS");
        assert_eq!(extraction.marks.len(), 2);
        assert_eq!(extraction.marks[0].event, "100mH");
    }
}
