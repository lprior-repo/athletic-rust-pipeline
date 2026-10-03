use super::discovery::{discover, MAX_HTML_BYTES, MAX_URL_BYTES};
use super::*;

mod acquisition;
mod foreign;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn capture(body: &[u8], requested: &str, observed: Option<&str>) -> FetchOutcome {
    FetchOutcome {
        url: requested.to_string(),
        response_url: observed.map(str::to_string),
        method: "GET".to_string(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-02T12:00:00Z".to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body: body.to_vec(),
    }
}

fn selected(html: &str) -> TestResult<String> {
    Ok(discover(
        &capture(html.as_bytes(), DIRECTORY_URL, None),
        &Url::parse(DIRECTORY_URL)?,
    )?
    .to_string())
}

fn refused(body: &[u8]) -> TestResult {
    match discover(
        &capture(body, DIRECTORY_URL, None),
        &Url::parse(DIRECTORY_URL)?,
    ) {
        Err(CrawlError::Schema { url, .. }) => check!(eq; url, DIRECTORY_URL),
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("unsafe or incomplete entry cannot select a bundle".into()),
    }
    Ok(())
}

#[test]
fn a_redeployment_changes_the_selected_asset_without_a_pinned_fallback() -> TestResult {
    for name in ["index-prior.js", "index-redeployed.js", "index-DdctX9sz.js"] {
        check!(eq;
            selected(&format!("<script type=module src=assets/{name}></script>"))?,
            format!("https://live.arbiter.io/directory/assets/{name}")
        );
    }
    Ok(())
}

#[test]
fn inert_malicious_tags_cannot_create_ambiguity_or_change_the_base() -> TestResult {
    let fake = "<base href='https://foreign.invalid/'><script type=module src='https://foreign.invalid/steal.js'></script>";
    let html = format!(
        "<!--{fake}--><template>{fake}<template>{fake}</template></template>\
         <svg>{fake}</svg><math>{fake}</math><noscript>{fake}</noscript>\
         <style>{fake}</style><textarea>{fake}</textarea><title>{fake}</title>\
         <script>const bait = \"<script type=module src='https://foreign.invalid/no.js'>\";</script>\
         <div data-bait=\"<script type=module src='https://foreign.invalid/no.js'>\"></div>\
         <link rel=modulepreload href='https://foreign.invalid/no.js'>\
         <script type=module>import 'https://foreign.invalid/no.js';</script>\
         <script TYPE=' Module ' SRC='assets/index-&#114;edeploy.js'></script>"
    );
    check!(eq;
        selected(&html)?,
        "https://live.arbiter.io/directory/assets/index-redeploy.js"
    );
    Ok(())
}

#[test]
fn decoded_attributes_and_first_duplicate_attributes_select_the_same_safe_module() -> TestResult {
    let html = "<SCRIPT SRC=' /directory/assets/index-&#65;.js ' src='https://foreign.invalid/no.js'\n TYPE=module type=text/javascript></SCRIPT>";
    check!(eq;
        selected(html)?,
        "https://live.arbiter.io/directory/assets/index-A.js"
    );
    Ok(())
}

#[test]
fn only_the_first_active_base_href_changes_subsequent_relative_modules() -> TestResult {
    let html = "<base target=_blank><template><base href='https://foreign.invalid/'></template><base href='/directory/assets/'><base href='https://foreign.invalid/'><script type=module src=index-next.js></script>";
    check!(eq;
        selected(html)?,
        "https://live.arbiter.io/directory/assets/index-next.js"
    );
    refused(b"<base href='https://foreign.invalid/'><base href='/directory/'><script type=module src='/directory/assets/index-next.js'></script>")?;
    Ok(())
}

#[test]
fn a_later_base_does_not_retroactively_change_an_earlier_module() -> TestResult {
    let html = "<script type=module src=assets/index-first.js></script><base href='/directory/assets/'><script type=module src=index-first.js></script>";
    check!(eq;
        selected(html)?,
        "https://live.arbiter.io/directory/assets/index-first.js"
    );
    Ok(())
}

#[test]
fn a_duplicate_resolved_module_is_unique_but_distinct_modules_are_refused() -> TestResult {
    check!(eq;
        selected("<script type=module src=assets/index-one.js></script><script type=module src=/directory/assets/index-one.js></script>")?,
        "https://live.arbiter.io/directory/assets/index-one.js"
    );
    refused(b"<script type=module src=assets/index-one.js></script><script type=module src=assets/index-two.js></script>")?;
    Ok(())
}

#[test]
fn the_observed_response_url_controls_resolution_without_rewriting_historical_provenance(
) -> TestResult {
    let authority = Url::parse(DIRECTORY_URL)?;
    let html = b"<script type=module src=../assets/index-final.js></script>";
    let observed = "https://live.arbiter.io/directory/shell/index.html";
    let entry = capture(html, DIRECTORY_URL, Some(observed));
    check!(eq;
        discover(&entry, &authority)?.as_str(),
        "https://live.arbiter.io/directory/assets/index-final.js"
    );
    let historical = capture(
        b"<script type=module src=assets/index-history.js></script>",
        DIRECTORY_URL,
        None,
    );
    check!(eq;
        discover(&historical, &authority)?.as_str(),
        "https://live.arbiter.io/directory/assets/index-history.js"
    );
    let foreign = capture(
        html,
        DIRECTORY_URL,
        Some("https://foreign.invalid/directory/"),
    );
    check!(matches!(
        discover(&foreign, &authority),
        Err(CrawlError::Schema { .. })
    ));
    Ok(())
}

#[test]
fn unsafe_authorities_schemes_ports_credentials_and_paths_are_refused() -> TestResult {
    for src in [
        "https://foreign.invalid/directory/assets/index.js",
        "//foreign.invalid/directory/assets/index.js",
        "http://live.arbiter.io/directory/assets/index.js",
        "https://live.arbiter.io:444/directory/assets/index.js",
        "https://user:password@live.arbiter.io/directory/assets/index.js",
        "javascript:alert(1)",
        "data:text/javascript,anything",
        "file:///directory/assets/index.js",
        "https://[bad/directory/assets/index.js",
        "/directory/assets/../../outside.js",
        "/directory/assets/sub/index.js",
        "/directory/assets/index%2fother.js",
        "/directory/assets/index.css",
        "/directory/assets/.js",
        "/directory/assets/index.js?probe=1",
        "/directory/assets/index.js#probe",
        "/directory/assets/in&#10;dex.js",
        "\\\\foreign.invalid\\directory\\assets\\index.js",
        "",
        "#index.js",
        "?index.js",
    ] {
        refused(format!("<script type=module src='{src}'></script>").as_bytes())?;
    }
    check!(eq;
        selected(
            "<script type=module src='//live.arbiter.io/directory/assets/index-safe.js'></script>"
        )?,
        "https://live.arbiter.io/directory/assets/index-safe.js"
    );
    Ok(())
}

#[test]
fn missing_non_utf8_and_truncated_declarations_cannot_select_a_bundle() -> TestResult {
    for body in [
        b"".as_slice(),
        b"<script src=assets/index.js></script>",
        b"<script type=module></script>",
        b"<script type=module src='assets/index.js'",
        b"<script type=module src='assets/index.js'>",
        b"<script type=module src='assets/in\xffdex.js'></script>",
        b"<script type=module src='assets/in\0dex.js'></script>",
    ] {
        refused(body)?;
    }
    Ok(())
}

#[test]
fn html_url_script_base_and_token_bounds_refuse_truncation_instead_of_partial_success() -> TestResult
{
    let module = "<script type=module src=assets/index-bound.js></script>";
    let exact = format!("{module}{}", " ".repeat(MAX_HTML_BYTES - module.len()));
    check!(eq;
        selected(&exact)?,
        "https://live.arbiter.io/directory/assets/index-bound.js"
    );
    refused(format!("{exact} ").as_bytes())?;
    let oversized = format!(
        "<script type=module src='assets/{}.js'></script>",
        "x".repeat(MAX_URL_BYTES)
    );
    refused(oversized.as_bytes())?;
    check!(eq;
        selected(&format!("{}{}", "<script></script>".repeat(127), module))?,
        "https://live.arbiter.io/directory/assets/index-bound.js"
    );
    refused(format!("{}{}", "<script></script>".repeat(128), module).as_bytes())?;
    check!(eq;
        selected(&format!("{}{}", "<base>".repeat(32), module))?,
        "https://live.arbiter.io/directory/assets/index-bound.js"
    );
    refused(format!("{}{}", "<base>".repeat(33), module).as_bytes())?;
    refused(format!("{module}{}", "<i></i>".repeat(4096)).as_bytes())?;
    Ok(())
}

#[test]
fn an_empty_or_fragment_base_href_is_still_the_first_authoritative_base() -> TestResult {
    for href in ["", "#directory", "?view=directory"] {
        check!(eq;
            selected(&format!("<base href='{href}'><base href='/directory/assets/'><script type=module src='assets/index-first-base.js'></script>"))?,
            "https://live.arbiter.io/directory/assets/index-first-base.js"
        );
    }
    Ok(())
}

#[test]
fn a_bundle_redirect_cannot_supply_credentials_from_a_foreign_or_non_asset_response() -> TestResult
{
    let authority = Url::parse(DIRECTORY_URL)?;
    let requested = "https://live.arbiter.io/directory/assets/index-owned.js";
    for observed in [
        "https://foreign.invalid/directory/assets/index-owned.js",
        "http://live.arbiter.io/directory/assets/index-owned.js",
        "https://live.arbiter.io/directory/login",
    ] {
        let bundle = capture(b"export const fixture = true;", requested, Some(observed));
        check!(matches!(
            super::discovery::validate_bundle(&bundle, &authority),
            Err(CrawlError::Schema { .. })
        ));
    }
    let historical = capture(b"export const fixture = true;", requested, None);
    super::discovery::validate_bundle(&historical, &authority)?;
    Ok(())
}
