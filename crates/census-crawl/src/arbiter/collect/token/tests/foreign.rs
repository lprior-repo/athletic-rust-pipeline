use super::{refused, selected, TestResult};

const REAL: &str = "<script type=module src=assets/index-real.js></script>";
const URL: &str = "https://live.arbiter.io/directory/assets/index-real.js";

#[test]
fn unmatched_foreign_closers_never_activate_inert_modules() -> TestResult {
    for inert in [
        "<svg></math><script type=module src=assets/index-decoy.js></script></svg>",
        "<math></svg><script type=module src=assets/index-decoy.js></script></math>",
    ] {
        refused(inert.as_bytes())?;
        check!(eq; selected(&format!("{inert}{REAL}"))?, URL);
    }
    Ok(())
}

#[test]
fn self_closing_foreign_roots_and_descendants_preserve_the_real_module() -> TestResult {
    for inert in [
        "<svg/>",
        "<math/>",
        "<svg><svg/><g/></svg>",
        "<math><math/><mi/></math>",
        "<svg><script type=module src='assets/index-decoy.js'/></svg>",
        "<svg><math></svg>",
    ] {
        check!(eq; selected(&format!("{inert}{REAL}"))?, URL);
    }
    Ok(())
}

#[test]
fn html_self_closing_slashes_do_not_close_template_or_script_scope() -> TestResult {
    refused(format!("<template/>{REAL}").as_bytes())?;
    refused(b"<script type=module src='assets/index-real.js'/>")?;
    check!(eq;
        selected(&format!("<template/>{REAL}</template>{REAL}"))?,
        URL
    );
    Ok(())
}

#[test]
fn excessive_inert_container_nesting_is_refused_before_selection() -> TestResult {
    let within_budget = format!("{}{}{REAL}", "<svg>".repeat(128), "</svg>".repeat(128));
    check!(eq; selected(&within_budget)?, URL);
    refused(format!("{}{}{REAL}", "<svg>".repeat(129), "</svg>".repeat(129)).as_bytes())?;
    Ok(())
}
