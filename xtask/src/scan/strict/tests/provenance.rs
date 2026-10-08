use super::*;

fn imported(definition: String, caller: String) -> anyhow::Result<Findings> {
    let root = PathBuf::from("/project/src/lib.rs");
    let sources = vec![
        (
            root.clone(),
            "#[path=\"macros.rs\"] mod macros; #[path=\"caller.rs\"] mod caller;".to_string(),
        ),
        (PathBuf::from("/project/src/macros.rs"), definition),
        (PathBuf::from("/project/src/caller.rs"), caller),
    ];
    inspect_sources(&sources, &[root], &std::collections::BTreeSet::new())
}

#[test]
fn imported_handwritten_expression_macro_traverses_real_closure_arguments() -> TestResult {
    let definition = "macro_rules! row { ($($value:expr),+ $(,)?) => { vec![$($crate::macros::cell($value)),+] }; } pub(crate) use row;";
    let caller = format!(
        "use crate::macros::row as values; fn run() {{ values!(|| {{{}}}); }}",
        "step();".repeat(59)
    );
    let found = imported(definition.to_string(), caller)?;
    check!(eq; found.callables, 2);
    check!(eq; found.over_60.len(), 2);
    check!(eq; found.unresolved.len(), 0);
    check!(validate_report(&report(found)).is_err());
    Ok(())
}

#[test]
fn imported_handwritten_standard_name_never_exempts_its_callable_template() -> TestResult {
    let definition = format!(
        "macro_rules! format {{ () => {{ fn generated() {{{}}} }}; }} pub(crate) use format;",
        "step();".repeat(59)
    );
    let caller = "use crate::macros::format; fn run() { format!(); }";
    let found = imported(definition, caller.to_string())?;
    check!(eq; found.callables, 2);
    check!(eq; found.over_60.len(), 1);
    check!(eq; found.unresolved.len(), 0);
    check!(validate_report(&report(found)).is_err());
    Ok(())
}

#[test]
fn unknown_imported_handwritten_template_remains_fail_closed() -> TestResult {
    let definition =
        "macro_rules! make { ($name:ident) => { fn $name() {} }; } pub(crate) use make;";
    let found = imported(
        definition.to_string(),
        "use crate::macros::make; make!(run);".to_string(),
    )?;
    check!(eq; found.callables, 0);
    check!(validate_report(&report(found)).is_err());
    Ok(())
}

#[test]
fn typed_template_fragments_keep_actual_generic_signature_arity() -> TestResult {
    let source = "macro_rules! make { ($type:ty, $value:expr) => { impl Trait for $type { fn six(&self,a:A,b:B,c:C,d:D,e:E) { let k = $value; } } }; } make!(Wrapper<A,B>, Kind::X);";
    let found = inspect(source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.parameters.len(), 1);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn repeating_handwritten_statements_in_callable_templates_is_not_measurable() -> TestResult {
    let found = inspect(
        "macro_rules! make { ($($body:expr),*) => { fn run() { $($body;)* } }; } make!(step());",
    )?;
    check!(validate_report(&report(found)).is_err());
    Ok(())
}

#[test]
fn known_vendor_imports_do_not_inherit_sibling_handwritten_shadowing() -> TestResult {
    let source = "use syn::Token; mod sibling { use opaque as futures; fn f() { futures::pin_mut!(x); } } fn real() { let _: Token![=>]; futures::pin_mut!(x); }";
    let found = inspect(source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 1);
    Ok(())
}

#[test]
fn macro_argument_local_imports_have_their_actual_lexical_scope() -> TestResult {
    let source = "fn run() { let a = vec![|| { use opaque::format; format!(); }]; format!(\"{}\", a.len()); }";
    let found = inspect(source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 1);
    Ok(())
}

#[test]
fn declaration_order_preserves_earlier_macro_shadowing() -> TestResult {
    let source = "macro_rules! format { ($name:ident) => { fn $name() {} }; } fn first() { format!(x); } macro_rules! format { () => { 1 }; } fn later() { format!(); }";
    let found = inspect(source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 2);
    Ok(())
}

#[test]
fn vendor_patterns_repetition_and_json_values_do_not_hide_handwritten_closures() -> TestResult {
    for invocation in [
        format!(
            "matches!((|| {{{}}})(), Some(value @ _))",
            "step();".repeat(59)
        ),
        format!("vec![|| {{{}}}; 2]", "step();".repeat(59)),
        format!(
            "serde_json::json!({{\"call\": || {{{}}}}})",
            "step();".repeat(59)
        ),
        format!(
            "tokio::select! {{ x = (|| {{{}}})() => x, else => 0 }}",
            "step();".repeat(59)
        ),
    ] {
        let found = inspect(&format!("fn run() {{ {invocation}; }}"))?;
        check!(eq; found.callables, 2);
        check!(eq; found.over_60.len(), 2);
        check!(eq; found.unresolved.len(), 0);
    }
    Ok(())
}

#[test]
fn raw_and_textually_imported_macros_do_not_inherit_standard_data_exemptions() -> TestResult {
    for prefix in [
        "macro_rules! r#stringify { ($value:expr) => { $value }; }",
        "#[macro_use] mod local { macro_rules! stringify { ($value:expr) => { $value }; } }",
    ] {
        let source = format!(
            "{prefix} fn run() {{ stringify!(|| {{{}}}); }}",
            "step();".repeat(59)
        );
        let found = inspect(&source)?;
        check!(eq; found.callables, 2);
        check!(eq; found.over_60.len(), 2);
        check!(eq; found.unresolved.len(), 0);
        check!(validate_report(&report(found)).is_err());
    }
    Ok(())
}
