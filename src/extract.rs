use crate::{
    config::OllamaConfig,
    marks,
    model::{Candidate, Mark, ModelDecision, Prospect, SearchHit},
};
use anyhow::{Context, Result};
use futures::TryStreamExt;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{sync::LazyLock, time::Duration};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub const AI_SCHEMA_VERSION: u32 = 6;
const MAX_MODEL_RESPONSE_BYTES: usize = 64 * 1024;

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

fn missing_evidence<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let Some(value) = Option::<T>::deserialize(deserializer)? else {
        return Ok(T::default());
    };
    Ok(value)
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct CandidateExtraction {
    #[serde(deserialize_with = "missing_evidence")]
    athlete_name: String,
    #[serde(deserialize_with = "missing_evidence")]
    school: String,
    #[serde(deserialize_with = "missing_evidence")]
    location: String,
    #[serde(deserialize_with = "missing_evidence")]
    graduation_year: Option<i32>,
    #[serde(deserialize_with = "missing_evidence")]
    sports: Vec<String>,
    #[serde(deserialize_with = "missing_evidence")]
    marks: Vec<ExtractedMark>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ExtractedMark {
    #[serde(default, deserialize_with = "missing_evidence")]
    event: String,
    #[serde(default, deserialize_with = "missing_evidence")]
    mark: String,
    #[serde(default, deserialize_with = "missing_evidence")]
    season: String,
    #[serde(default, deserialize_with = "missing_evidence")]
    date: String,
    #[serde(default, deserialize_with = "missing_evidence")]
    meet_name: String,
    #[serde(default, deserialize_with = "missing_evidence")]
    wind: Option<String>,
    #[serde(default)]
    is_pr: bool,
}
#[derive(Serialize)]
struct ProspectSummary<'a> {
    first_name: &'a str,
    last_name: &'a str,
    school: &'a str,
    city: &'a str,
    state: &'a str,
    street: &'a str,
    postal_code: &'a str,
    parsed_address: crate::address::AddressEvidence,
    source_sport: &'a str,
    expected_graduation_year: Option<i32>,
}

impl<'a> From<&'a Prospect> for ProspectSummary<'a> {
    fn from(prospect: &'a Prospect) -> Self {
        Self {
            first_name: &prospect.first_name,
            last_name: &prospect.last_name,
            school: &prospect.school,
            city: &prospect.city,
            state: &prospect.state,
            street: prospect
                .source_fields
                .get("Address Mailing / Permanent Street Combined")
                .map_or("", String::as_str),
            postal_code: prospect
                .source_fields
                .get("Address Mailing / Permanent Postal")
                .map_or("", String::as_str),
            parsed_address: crate::address::parse(prospect),
            source_sport: &prospect.sport,
            expected_graduation_year: prospect.expected_graduation_year,
        }
    }
}

#[derive(Debug, Serialize)]
struct CandidateSummary<'a> {
    index: usize,
    profile_url: &'a str,
    search_title: &'a str,
    search_snippet: &'a str,
    athlete_name: &'a str,
    school: &'a str,
    location: &'a str,
    graduation_year: Option<i32>,
    sports: &'a [String],
    evidence_text: &'a str,
    evidence_urls: &'a [String],
    marks: &'a [Mark],
    deterministic_score: f64,
    corroborated: bool,
}
#[derive(Debug, Deserialize, Default)]
struct IdentityDecisionPayload {
    #[serde(default)]
    decision: Option<String>,
    #[serde(default)]
    candidate_index: Option<usize>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    track_confirmed: bool,
    #[serde(default)]
    xc_confirmed: bool,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    model_status: Option<String>,
}

impl IdentityDecisionPayload {
    fn into_model_decision(self) -> Result<ModelDecision> {
        Ok(ModelDecision {
            decision: self.decision.context("identity model omitted decision")?,
            candidate_index: self.candidate_index,
            confidence: self
                .confidence
                .context("identity model omitted confidence")?,
            track_confirmed: self.track_confirmed,
            xc_confirmed: self.xc_confirmed,
            reason: self.reason,
            model_status: match self.model_status {
                Some(value) => value,
                None => "ok".to_owned(),
            },
        })
    }
}

#[derive(Clone, Copy)]
enum IdentityReviewMode {
    Legacy,
    Exhaustive,
}

pub(crate) async fn bounded_response_body(response: reqwest::Response) -> Result<Vec<u8>> {
    let max_bytes = u64::try_from(MAX_MODEL_RESPONSE_BYTES)
        .context("model response size limit does not fit in u64")?;
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes)
    {
        anyhow::bail!("local model response exceeds {MAX_MODEL_RESPONSE_BYTES} bytes");
    }
    response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |body, chunk| async move {
            let next_length = body
                .len()
                .checked_add(chunk.len())
                .context("local model response size overflow")?;
            if next_length > MAX_MODEL_RESPONSE_BYTES {
                anyhow::bail!("local model response exceeds {MAX_MODEL_RESPONSE_BYTES} bytes");
            }
            let mut body = body;
            body.try_reserve(chunk.len())
                .context("allocating local model response buffer")?;
            body.extend_from_slice(&chunk);
            Ok(body)
        })
        .await
        .context("reading local model response body")
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

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
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
        let response_body =
            crate::model_transport::post_json(&self.client, &endpoint, &body).await?;
        let content = if openai_compatible {
            serde_json::from_slice::<OpenAiResponse>(&response_body)
                .context("decoding OpenAI-compatible local model response")?
                .choices
                .into_iter()
                .next()
                .map(|choice| choice.message.content)
                .filter(|value| !value.trim().is_empty())
                .context("local model returned no text choice")?
        } else {
            serde_json::from_slice::<OllamaResponse>(&response_body)
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
        let prospect_json = serde_json::to_string(&ProspectSummary::from(prospect))
            .context("serializing postal identity context")?;
        let prompt = format!(
            r#"Extract only facts explicitly supported by the candidate evidence.

Prospect context is supplied only to focus extraction, not as evidence:
{prospect_json}
The full mailing/permanent address is context, not proof of the athlete's residence.
Never copy its street, city, state or postal code into candidate facts without independent evidence.
Parsed address fields describe source syntax only; they do not validate postal deliverability or prove residence.

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
            prospect_json = prospect_json,
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
        match self
            .request_identity(prospect, candidates, IdentityReviewMode::Legacy)
            .await
        {
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

    pub async fn validate_identity_required(
        &self,
        prospect: &Prospect,
        candidates: &[Candidate],
    ) -> Result<ModelDecision> {
        let decision = self
            .request_identity(prospect, candidates, IdentityReviewMode::Exhaustive)
            .await?;
        validate_model_decision(&decision, candidates)?;
        Ok(decision)
    }

    async fn request_identity(
        &self,
        prospect: &Prospect,
        candidates: &[Candidate],
        mode: IdentityReviewMode,
    ) -> Result<ModelDecision> {
        if !self.enabled {
            anyhow::bail!("Local identity reviewer is disabled");
        }
        let summaries: Vec<CandidateSummary<'_>> = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| CandidateSummary {
                index,
                profile_url: &candidate.profile_url,
                search_title: &candidate.search_title,
                search_snippet: &candidate.search_snippet,
                athlete_name: &candidate.athlete_name,
                school: &candidate.school,
                location: &candidate.location,
                graduation_year: candidate.graduation_year,
                sports: &candidate.sports,
                evidence_text: &candidate.evidence_text,
                evidence_urls: &candidate.evidence_urls,
                marks: &candidate.marks,
                deterministic_score: candidate.deterministic_score,
                corroborated: candidate.corroborated,
            })
            .collect();
        let prospect_summary = ProspectSummary::from(prospect);
        let prospect_json = serde_json::to_string(&prospect_summary)
            .context("serializing permitted prospect identity context")?;
        let summaries_json =
            serde_json::to_string_pretty(&summaries).context("serializing candidate context")?;
        let prompt = match mode {
            IdentityReviewMode::Legacy => format!(
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
            ),
            IdentityReviewMode::Exhaustive => format!(
                r#"Perform conservative identity review using only the supplied candidate evidence. Exact name alone is insufficient; independently corroborating school, geography, or class year is required when available. Conflicting candidate school, state, or year is negative evidence. Do not invent facts.

The prospect's sport is a source-row provenance field, not an identity constraint. Do not reject a candidate solely because its sports differ from that source field, and do not require a candidate sport to match it. Track & Field and Cross Country are independent candidate evidence: confirm each only when the candidate evidence supports it. Never copy prospect fields into candidate facts or rewrite candidate sports.
The full mailing/permanent postal address can distinguish same-name people. Compare geography only against independently supported candidate facts. A missing candidate street/postal address is unknown, not a mismatch. A conflicting state is negative evidence; a move is possible but must not be invented. Never infer identity from the name alone.

Prospect:
{prospect_json}

Candidates (zero-based index; summaries include the candidate's actual evidence):
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
            ),
        };
        self.chat_json::<IdentityDecisionPayload>(&prompt)
            .await?
            .into_model_decision()
    }
}

pub fn validate_model_decision(decision: &ModelDecision, candidates: &[Candidate]) -> Result<()> {
    if decision.model_status != "ok" {
        anyhow::bail!(
            "identity reviewer returned invalid model_status {}: {}",
            decision.model_status,
            decision.reason
        );
    }
    if !decision.confidence.is_finite() || !(0.0..=1.0).contains(&decision.confidence) {
        anyhow::bail!("identity reviewer returned invalid confidence");
    }
    if !matches!(
        decision.decision.as_str(),
        "MATCH" | "CLOSE_MATCH" | "REVIEW" | "NO_MATCH"
    ) {
        anyhow::bail!(
            "identity reviewer returned invalid decision {}",
            decision.decision
        );
    }
    if let Some(index) = decision.candidate_index {
        if candidates.get(index).is_none() {
            anyhow::bail!("identity reviewer returned invalid candidate index {index}");
        }
    }
    if matches!(decision.decision.as_str(), "MATCH" | "CLOSE_MATCH")
        && decision.candidate_index.is_none()
    {
        anyhow::bail!(
            "identity reviewer decision {} requires a candidate index",
            decision.decision
        );
    }
    Ok(())
}

pub async fn candidate_from_evidence_required(
    prospect: &Prospect,
    hit: &SearchHit,
    html: Option<&str>,
    ollama: &OllamaClient,
    page_text_limit: usize,
) -> Result<Candidate> {
    let evidence = build_evidence(hit, html, page_text_limit);
    let extraction = ollama
        .extract_candidate(prospect, hit, &evidence)
        .await
        .map_err(|error| {
            anyhow::anyhow!("required local-model candidate extraction failed: {error:#}")
        })?;
    Ok(candidate_from_extraction(hit, html, extraction, evidence))
}

pub fn candidate_from_evidence_deterministic(
    prospect: &Prospect,
    hit: &SearchHit,
    html: Option<&str>,
    page_text_limit: usize,
) -> Candidate {
    let evidence = build_evidence(hit, html, page_text_limit);
    let mut extraction = fallback_extraction(hit);
    enrich_from_search_evidence(prospect, hit, &mut extraction);
    extraction.sports.retain(|sport| {
        (sport == "Track & Field" && hit.url.contains("track-and-field"))
            || (sport == "Cross Country" && hit.url.contains("cross-country"))
    });
    let geographic_prefix = CLASS_YEAR
        .as_ref()
        .and_then(|pattern| pattern.find(&hit.snippet))
        .and_then(|matched| hit.snippet.get(..matched.start()))
        .map_or(hit.snippet.as_str(), |value| value);
    let state = crate::scoring::location_state(
        geographic_prefix.trim_end_matches(|c: char| !c.is_alphabetic()),
    )
    .or_else(|| {
        hit.snippet.split_whitespace().find_map(|token| {
            let token = token.trim_matches(|c: char| !c.is_alphabetic());
            (token.len() == 2 && token.chars().all(|c| c.is_ascii_uppercase()))
                .then(|| crate::alpha_url::canonical_state(token))
                .flatten()
        })
    });
    if let Some(state) = state {
        extraction.location = state;
    }
    extraction.graduation_year = CLASS_YEAR
        .as_ref()
        .and_then(|pattern| pattern.captures(&evidence)?.get(1)?.as_str().parse().ok());
    candidate_from_extraction(hit, html, extraction, evidence)
}

static CLASS_YEAR: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:class\s+of|graduation(?:\s+year)?)\s*:?\s*(20\d{2})\b").ok()
});

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
    candidate_from_extraction(hit, html, extraction, evidence)
}

pub fn candidate_evidence(hit: &SearchHit, html: Option<&str>, page_text_limit: usize) -> String {
    build_evidence(hit, html, page_text_limit)
}

fn candidate_from_extraction(
    hit: &SearchHit,
    html: Option<&str>,
    extraction: CandidateExtraction,
    evidence: String,
) -> Candidate {
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
    let observed = normalize_text(text);
    let canonical = match crate::alpha_url::canonical_state(&prospect.state) {
        Some(state) => state,
        None => {
            let city = prospect.city.trim();
            let region = prospect.state.trim();
            let expected = normalize_text(&format!("{city} {region}"));
            return (!city.is_empty()
                && !region.is_empty()
                && contains_phrase(&observed, &expected))
            .then(|| format!("{city}, {region}"));
        }
    };
    let full_state = normalize_text(state_name_to_full(&canonical));
    let explicit_code = text
        .split_whitespace()
        .any(|word| word.trim_matches(|c: char| !c.is_alphabetic()) == canonical);
    if !explicit_code && !contains_phrase(&observed, &full_state) {
        return None;
    }
    let city = normalize_text(&prospect.city);
    if contains_phrase(&observed, &city) {
        Some(format!("{}, {canonical}", prospect.city.trim()))
    } else {
        // A state match must not manufacture the prospect's city.
        Some(canonical)
    }
}

fn contains_phrase(text: &str, phrase: &str) -> bool {
    !phrase.is_empty()
        && text.match_indices(phrase).any(|(start, matched)| {
            let boundary_before = start == 0
                || text
                    .get(..start)
                    .is_some_and(|prefix| prefix.ends_with(' '));
            let boundary_after = start
                .checked_add(matched.len())
                .and_then(|end| text.get(end..))
                .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with(' '));
            boundary_before && boundary_after
        })
}

fn state_name_to_full(abbrev: &str) -> &str {
    match abbrev.trim().to_uppercase().as_str() {
        "AL" => "Alabama",
        "AK" => "Alaska",
        "AZ" => "Arizona",
        "AR" => "Arkansas",
        "CA" => "California",
        "CO" => "Colorado",
        "CT" => "Connecticut",
        "DE" => "Delaware",
        "FL" => "Florida",
        "GA" => "Georgia",
        "HI" => "Hawaii",
        "ID" => "Idaho",
        "IL" => "Illinois",
        "IN" => "Indiana",
        "IA" => "Iowa",
        "KS" => "Kansas",
        "KY" => "Kentucky",
        "LA" => "Louisiana",
        "ME" => "Maine",
        "MD" => "Maryland",
        "MA" => "Massachusetts",
        "MI" => "Michigan",
        "MN" => "Minnesota",
        "MS" => "Mississippi",
        "MO" => "Missouri",
        "MT" => "Montana",
        "NE" => "Nebraska",
        "NV" => "Nevada",
        "NH" => "New Hampshire",
        "NJ" => "New Jersey",
        "NM" => "New Mexico",
        "NY" => "New York",
        "NC" => "North Carolina",
        "ND" => "North Dakota",
        "OH" => "Ohio",
        "OK" => "Oklahoma",
        "OR" => "Oregon",
        "PA" => "Pennsylvania",
        "RI" => "Rhode Island",
        "SC" => "South Carolina",
        "SD" => "South Dakota",
        "TN" => "Tennessee",
        "TX" => "Texas",
        "UT" => "Utah",
        "VT" => "Vermont",
        "VA" => "Virginia",
        "WA" => "Washington",
        "WV" => "West Virginia",
        "WI" => "Wisconsin",
        "WY" => "Wyoming",
        _ => "",
    }
}

fn name_before_location(title: &str, location: &str) -> Option<String> {
    let title_lower = title.to_ascii_lowercase();
    let location_lower = location.to_ascii_lowercase();
    if let Some(end) = title_lower.find(&location_lower) {
        let name = title.get(..end)?.trim().trim_end_matches([',', '-', ' ']);
        let name = name.split("...").next()?.trim();
        let token_count = name.split_whitespace().count();
        if name.len() >= 2 && token_count <= 5 {
            return Some(name.to_owned());
        }
    }
    let city_words: Vec<&str> = location_lower
        .split_whitespace()
        .filter(|w| w.len() >= 3)
        .collect();
    if !city_words.is_empty() {
        for word in &city_words {
            if let Some(pos) = title_lower.rfind(word) {
                if pos >= word.len() {
                    let name = title.get(..pos.saturating_sub(1))?.trim();
                    let name = name.trim_end_matches([',', '-', ' ', '.']);
                    let name = name.split("...").next()?.trim();
                    let token_count = name.split_whitespace().count();
                    if name.len() >= 2 && token_count <= 5 {
                        return Some(name.to_owned());
                    }
                }
            }
        }
    }
    None
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

    #[test]
    fn null_optional_extraction_fields_remain_empty() -> Result<()> {
        let extraction: CandidateExtraction = serde_json::from_str(
            r#"{
                "athlete_name": null,
                "school": null,
                "location": null,
                "graduation_year": null,
                "sports": null,
                "marks": null
            }"#,
        )?;
        assert!(extraction.athlete_name.is_empty());
        assert!(extraction.school.is_empty());
        assert!(extraction.location.is_empty());
        assert!(extraction.graduation_year.is_none());
        assert!(extraction.sports.is_empty());
        assert!(extraction.marks.is_empty());
        Ok(())
    }

    #[test]
    fn strict_identity_validation_rejects_invalid_confidence() -> Result<()> {
        let decision = ModelDecision {
            decision: "MATCH".to_owned(),
            candidate_index: Some(0),
            confidence: 1.1,
            model_status: "ok".to_owned(),
            ..Default::default()
        };
        let error = match validate_model_decision(&decision, &[Candidate::default()]) {
            Ok(()) => anyhow::bail!("invalid confidence unexpectedly accepted"),
            Err(error) => error,
        };
        assert!(format!("{error:#}").contains("invalid confidence"));
        Ok(())
    }

    #[tokio::test]
    async fn required_candidate_extraction_rejects_disabled_model() -> Result<()> {
        let client = OllamaClient::new(&OllamaConfig {
            api: "openai-compatible".to_owned(),
            enabled: false,
            url: "http://127.0.0.1:1".to_owned(),
            model: "test-model".to_owned(),
            timeout_seconds: 1,
        })?;
        let prospect = Prospect {
            first_name: "Ada".to_owned(),
            last_name: "Runner".to_owned(),
            ..Default::default()
        };
        let hit = SearchHit {
            url: "https://www.athletic.net/athlete/7/track-and-field".to_owned(),
            title: "Ada Runner".to_owned(),
            ..Default::default()
        };
        let error =
            match candidate_from_evidence_required(&prospect, &hit, None, &client, 4_000).await {
                Ok(_) => anyhow::bail!("required AI extraction unexpectedly succeeded"),
                Err(error) => error,
            };
        assert!(format!("{error:#}").contains("Local model is disabled"));
        Ok(())
    }

    #[tokio::test]
    async fn omitted_extraction_schema_is_an_error_not_empty_evidence() -> Result<()> {
        let mut server = mockito::Server::new_async().await;
        let _response = server
            .mock("POST", "/v1/chat/completions")
            .with_body(json!({"choices": [{"message": {"content": "{}"}}]}).to_string())
            .create_async()
            .await;
        let client = OllamaClient::new(&OllamaConfig {
            api: "openai-compatible".to_owned(),
            enabled: true,
            url: server.url(),
            model: "fixture".to_owned(),
            timeout_seconds: 5,
        })?;
        let hit = SearchHit {
            url: "https://www.athletic.net/athlete/7/track-and-field".to_owned(),
            title: "Ada Runner".to_owned(),
            ..Default::default()
        };
        let result =
            candidate_from_evidence_required(&Prospect::default(), &hit, None, &client, 4_000)
                .await;
        let error = match result {
            Ok(_) => anyhow::bail!("missing extraction schema accepted as evidence"),
            Err(error) => error,
        };
        assert!(format!("{error:#}").contains("missing field"));
        Ok(())
    }
}
