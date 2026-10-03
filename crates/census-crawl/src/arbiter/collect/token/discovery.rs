use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};
use html5gum::Tokenizer;
use url::Url;

mod markup;

pub(super) const MAX_HTML_BYTES: usize = 65_536;
pub(super) const MAX_URL_BYTES: usize = 4_096;
const MAX_TOKENS: usize = 8_192;

pub(super) fn refuse(url: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: url.to_string(),
        detail: detail.to_string(),
    }
}

pub(super) fn discover(entry: &FetchOutcome, authority: &Url) -> CrawlResult<Url> {
    let requested = document_url(&entry.url, authority, &entry.url)?;
    let document = match entry.response_url.as_deref() {
        Some(observed) => document_url(observed, authority, &entry.url)?,
        None => requested,
    };
    if entry.body.len() > MAX_HTML_BYTES {
        return Err(refuse(&entry.url, "directory HTML exceeds 65536 bytes"));
    }
    let html = std::str::from_utf8(&entry.body)
        .map_err(|_| refuse(&entry.url, "directory HTML is not UTF-8"))?;
    if html.contains('\0') {
        return Err(refuse(&entry.url, "directory HTML contains a null byte"));
    }
    let initial = Selection {
        base: document.clone(),
        module: None,
        based: false,
    };
    let mut tokens = Tokenizer::new_with_emitter(html, markup::Markup::default());
    let selected = tokens
        .by_ref()
        .take(MAX_TOKENS)
        .try_fold(initial, |selection, token| {
            let token = token.map_err(|_| refuse(&entry.url, "directory HTML input failed"))?;
            selection.accept(token, &document, authority, &entry.url)
        })?;
    if tokens.next().is_some() {
        return Err(refuse(&entry.url, "directory HTML exceeds 8192 tokens"));
    }
    selected
        .module
        .ok_or_else(|| refuse(&entry.url, "no active external module in directory HTML"))
}

pub(super) fn validate_bundle(bundle: &FetchOutcome, authority: &Url) -> CrawlResult<()> {
    let requested = document_url(&bundle.url, authority, &bundle.url)?;
    module_path(&requested, &bundle.url)?;
    if let Some(observed) = bundle.response_url.as_deref() {
        let final_url = document_url(observed, authority, &bundle.url)?;
        module_path(&final_url, &bundle.url)?;
    }
    Ok(())
}

struct Selection {
    base: Url,
    module: Option<Url>,
    based: bool,
}

impl Selection {
    fn accept(
        mut self,
        token: markup::Declaration,
        document: &Url,
        authority: &Url,
        requested: &str,
    ) -> CrawlResult<Self> {
        match token {
            markup::Declaration::Invalid(detail) => return Err(refuse(requested, detail)),
            markup::Declaration::Base(href) if !self.based => {
                self.base = resolve(&href, document, authority, requested)?;
                self.based = true;
            }
            markup::Declaration::Module(src) => {
                let value = src.trim_ascii();
                if value.is_empty()
                    || value
                        .first()
                        .is_some_and(|byte| matches!(*byte, b'#' | b'?'))
                {
                    return Err(refuse(
                        requested,
                        "empty or non-resource directory module URL",
                    ));
                }
                let candidate = resolve(&src, &self.base, authority, requested)?;
                module_path(&candidate, requested)?;
                if self
                    .module
                    .as_ref()
                    .is_some_and(|previous| previous != &candidate)
                {
                    return Err(refuse(
                        requested,
                        "distinct active directory modules are ambiguous",
                    ));
                }
                self.module = Some(candidate);
            }
            markup::Declaration::Base(_) | markup::Declaration::Tick => {}
        }
        Ok(self)
    }
}

fn document_url(value: &str, authority: &Url, requested: &str) -> CrawlResult<Url> {
    clean_url(value, requested)?;
    let parsed =
        Url::parse(value).map_err(|_| refuse(requested, "invalid directory document URL"))?;
    same_origin(&parsed, authority, requested)?;
    Ok(parsed)
}

fn resolve(bytes: &[u8], base: &Url, authority: &Url, requested: &str) -> CrawlResult<Url> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| refuse(requested, "directory URL attribute is not UTF-8"))?
        .trim_matches(|ch: char| ch.is_ascii_whitespace());
    clean_url(value, requested)?;
    let resolved = base
        .join(value)
        .map_err(|_| refuse(requested, "invalid directory URL attribute"))?;
    same_origin(&resolved, authority, requested)?;
    Ok(resolved)
}

fn clean_url(value: &str, requested: &str) -> CrawlResult<()> {
    if value.len() > MAX_URL_BYTES {
        return Err(refuse(requested, "directory URL exceeds 4096 bytes"));
    }
    if value
        .bytes()
        .any(|byte| byte.is_ascii_control() || byte == b'\\')
    {
        return Err(refuse(
            requested,
            "directory URL contains unsafe control or separator bytes",
        ));
    }
    let authority = value
        .split_once("://")
        .map(|(_, rest)| rest)
        .or_else(|| value.strip_prefix("//"));
    if authority.is_some_and(|rest| {
        rest.trim_start_matches('/')
            .split(['/', '?', '#'])
            .next()
            .is_some_and(|host| host.contains('@'))
    }) {
        return Err(refuse(requested, "directory URL contains userinfo"));
    }
    Ok(())
}

fn same_origin(candidate: &Url, authority: &Url, requested: &str) -> CrawlResult<()> {
    if !matches!(candidate.scheme(), "http" | "https")
        || candidate.origin() != authority.origin()
        || !candidate.username().is_empty()
        || candidate.password().is_some()
        || candidate.as_str().len() > MAX_URL_BYTES
    {
        return Err(refuse(
            requested,
            "directory URL is not credential-free source-owned HTTP(S)",
        ));
    }
    Ok(())
}

fn module_path(candidate: &Url, requested: &str) -> CrawlResult<()> {
    let Some(filename) = candidate.path().strip_prefix("/directory/assets/") else {
        return Err(refuse(
            requested,
            "directory module is outside /directory/assets/",
        ));
    };
    if filename.len() <= 3
        || !filename.ends_with(".js")
        || filename.contains(['/', '%'])
        || candidate.query().is_some()
        || candidate.fragment().is_some()
    {
        return Err(refuse(
            requested,
            "directory module is not a direct JavaScript asset",
        ));
    }
    Ok(())
}
