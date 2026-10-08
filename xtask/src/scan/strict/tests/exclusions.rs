use super::*;

#[test]
fn captured_literals_do_not_create_functions_parameters_or_actions() -> TestResult {
    let source = format!("fn run() {{\nlet capture = r###\"\n{}fn fake(a,b,c,d,e,f) {{}} || {{ step(); }}\n\"###;\n/* outer /* nested */ done */\n}}", "record\n".repeat(80));
    let found = inspect(&source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.lines.len(), 0);
    check!(eq; found.parameters.len(), 0);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn test_item_exclusion_never_hides_later_production() -> TestResult {
    let source = format!(
        "#[cfg(test)] mod tests {{ {} }}\n{}",
        function(61),
        function(61)
    );
    let found = inspect(&source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.over_60.len(), 1);
    Ok(())
}

#[test]
fn cfg_exclusions_follow_boolean_structure_not_test_word_presence() -> TestResult {
    for cfg in ["test", "all(test, feature=\"x\")", "not(not(test))"] {
        check!(eq; inspect(&format!("#[cfg({cfg})] {}", function(61)))?.callables, 0);
    }
    for cfg in ["any(test,feature=\"production\")", "not(test)"] {
        check!(eq; inspect(&format!("#[cfg({cfg})] {}", function(61)))?.over_60.len(), 1);
    }
    Ok(())
}

#[test]
fn ordinary_impl_named_tests_is_production() -> TestResult {
    let found = inspect(&format!("impl tests {{ {} }}", function(61)))?;
    check!(eq; found.callables, 1);
    check!(eq; found.over_60.len(), 1);
    Ok(())
}

#[test]
fn captured_macro_tokens_are_data_not_callable_ast() -> TestResult {
    let found = inspect("fn run() { stringify!(fn fake(a,b,c,d,e,f) {}); }")?;
    check!(eq; found.callables, 1);
    check!(eq; found.parameters.len(), 0);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn compiler_generated_attributes_and_vendor_macros_are_style_excluded() -> TestResult {
    let source = "#[derive(serde::Serialize, clap::Parser)] #[serde(default)] struct X; #[arg(long)] struct Y; #[restate::service] trait Z {} #[tracing::instrument] fn run() { serde_json::json!({\"x\": 1}); println!(\"x\"); }";
    let found = inspect(source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn opaque_handwritten_macros_and_included_source_fail_closed() -> TestResult {
    for source in [
        "generate!();",
        "include!(\"generated.rs\");",
        "fn run() { custom::format!(); }",
        "macro_rules! make { ($name:ident) => { fn $name() {} }; } make!(x);",
    ] {
        let found = inspect(source)?;
        check!(!found.unresolved.is_empty());
        check!(validate_report(&report(found)).is_err());
    }
    Ok(())
}

#[test]
fn literal_handwritten_macro_templates_count_functions_and_arity() -> TestResult {
    let found =
        inspect("macro_rules! make { () => { fn six(a:A,b:B,c:C,d:D,e:E,f:F) {} }; } make!();")?;
    check!(eq; found.callables, 1);
    check!(eq; found.parameters.len(), 1);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn handwritten_macro_functions_and_closures_resist_packed_61_actions() -> TestResult {
    for steps in [58, 59] {
        for template in [
            format!("fn run() {{{}}}", "step();".repeat(steps)),
            format!("const F: X = || {{{}}};", "step();".repeat(steps)),
        ] {
            let source = format!("macro_rules! make {{ () => {{ {template} }}; }} make!();");
            let found = inspect(&source)?;
            check!(eq; found.callables, 1);
            check!(eq; found.unresolved.len(), 0);
            check!(eq; found.over_60.len(), usize::from(steps == 59));
            check!(eq; validate_report(&report(found)).is_err(), steps == 59);
        }
    }
    Ok(())
}

#[test]
fn macro_captured_literals_and_stringified_fixture_functions_remain_data() -> TestResult {
    let source = "macro_rules! data { () => { const S: &str = stringify!(fn fake(a,b,c,d,e,f) {}); }; } fn run() {} data!();";
    let found = inspect(source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.parameters.len(), 0);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn invalid_rust_never_produces_empty_certification() -> TestResult {
    for source in [
        "fn run() { let x = r###\"unterminated",
        "fn run(a:) {}",
        "fn run() { |x }",
        "fn run() { step(] }",
    ] {
        check!(inspect(source).is_err());
    }
    Ok(())
}

#[test]
fn real_if_not_pipes_and_unbraced_generic_closure_are_ast_not_macros() -> TestResult {
    let found = inspect("fn run() { if !(x | y > 0) { act(); } let f = || create::<A,B>(); }")?;
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 0);
    check!(eq; found.parameters.len(), 0);
    Ok(())
}

#[test]
fn nested_test_functions_do_not_inflate_outer_budget() -> TestResult {
    let found = inspect(&format!("fn outer() {{\n#[cfg(test)]\n{}}}", function(61)))?;
    check!(eq; found.callables, 1);
    check!(eq; found.lines.len(), 0);
    Ok(())
}

#[test]
fn test_associated_constants_and_test_locals_do_not_hide_neighbor_closures() -> TestResult {
    let source = "impl X { #[cfg(test)] const T: X = |a,b,c,d,e,f| f; const P: X = |a,b,c,d,e,f| f; } fn run() { #[cfg(test)] let t = |a,b,c,d,e,f| f; let p = || 1; }";
    let found = inspect(source)?;
    check!(eq; found.callables, 3);
    check!(eq; found.parameters.len(), 1);
    Ok(())
}

#[test]
fn compiler_inert_inner_attributes_are_not_macros() -> TestResult {
    let found = inspect("#![forbid(unsafe_code)] fn run() {}")?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn vendor_rust_arguments_measure_real_closures_but_stringify_is_data() -> TestResult {
    let source = format!(
        "fn run() {{ let v = vec![|| {{{}}}]; }}",
        "step();".repeat(59)
    );
    let found = inspect(&source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.over_60.len(), 2);
    let data = format!(
        "fn run() {{ stringify!(fn fake() {{\n{}\n}}); }}",
        "step();\n".repeat(80)
    );
    let captured = inspect(&data)?;
    check!(eq; captured.callables, 1);
    check!(eq; captured.lines.len(), 0);
    check!(eq; captured.unresolved.len(), 0);
    Ok(())
}

#[test]
fn handwritten_macro_shadowing_never_inherits_vendor_exemption() -> TestResult {
    for source in [
        "use custom::format; fn run() { format!(); }",
        "macro_rules! format { ($body:tt) => { $body }; } fn run() { format!(x); }",
    ] {
        let found = inspect(source)?;
        check!(!found.unresolved.is_empty());
        check!(validate_report(&report(found)).is_err());
    }
    let imported = inspect("use serde_json::json as data; fn run() { data!({\"x\": 1}); }")?;
    check!(eq; imported.unresolved.len(), 0);
    Ok(())
}

#[test]
fn namespace_aliases_preserve_vendor_and_handwritten_distinction() -> TestResult {
    let vendor = inspect("use serde_json as render; fn run() { render::json!({\"x\": 1}); }")?;
    check!(eq; vendor.unresolved.len(), 0);
    let shadow = inspect("use custom as serde_json; fn run() { serde_json::json!({\"x\": 1}); }")?;
    check!(eq; shadow.unresolved.len(), 1);
    Ok(())
}

#[test]
fn sibling_scope_macro_shadow_does_not_change_standard_macro_provenance() -> TestResult {
    let source = "mod custom { use opaque::format; fn f() { format!(); } } fn real() { format!(\"{}\", 1); }";
    let found = inspect(source)?;
    check!(eq; found.unresolved.len(), 1);
    Ok(())
}

#[test]
fn known_parent_glob_does_not_make_standard_macros_unknown() -> TestResult {
    let source = "struct X; mod child { use super::*; fn f() { let v = vec![X]; format!(\"{}\", v.len()); matches!(v.len(), 0); } }";
    let found = inspect(source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}
