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

#[test]
fn vendor_prelude_globs_do_not_invent_macro_exports_or_hide_closures() -> TestResult {
    for invocation in [
        "format!(\"{}\", (|| { BODY })())",
        "vec![|| { BODY }]",
        "matches!((|| { BODY })(), Some(_))",
        "tracing::warn!(value = (|| { BODY })(), \"observed\")",
        "anyhow::anyhow!(\"{}\", (|| { BODY })())",
    ] {
        let source = format!(
            "use restate_sdk::prelude::*; fn run() {{ {invocation}; cfg!(unix); }}",
            invocation = invocation.replace("BODY", &"step();".repeat(59))
        );
        let found = inspect(&source)?;
        check!(eq; found.callables, 2);
        check!(eq; found.over_60.len(), 2);
        check!(eq; found.unresolved.len(), 0);
    }
    Ok(())
}

#[test]
fn same_named_vendor_macro_imports_resolve_the_crate_prefix() -> TestResult {
    let found = inspect(
        "use anyhow::{anyhow, ensure}; fn run() { ensure!(true, \"valid\"); anyhow!(\"message\"); }",
    )?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn vendor_glob_cycles_leave_standard_fallback_and_vendor_aliases_resolvable() -> TestResult {
    let found = inspect(
        "use anyhow::*; use tracing as log; mod caller { use super::*; fn run() { format!(\"value\"); anyhow!(\"message\"); log::warn!(\"observed\"); } }",
    )?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn handwritten_standard_shadow_wins_over_unrelated_vendor_prelude() -> TestResult {
    let source = format!(
        "use restate_sdk::prelude::*; macro_rules! stringify {{ ($value:expr) => {{ $value }}; }} fn run() {{ stringify!(|| {{{}}}); }}",
        "step();".repeat(59)
    );
    let found = inspect(&source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.over_60.len(), 2);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn project_glob_reexports_shadow_prelude_and_preserve_callable_templates() -> TestResult {
    let source = format!(
        "use restate_sdk::prelude::*; mod definitions {{ macro_rules! format {{ () => {{ fn generated() {{{}}} }}; }} pub(crate) use format; }} mod reexports {{ pub(crate) use crate::definitions::*; }} use reexports::*; fn run() {{ format!(); }}",
        "step();".repeat(59)
    );
    let found = inspect(&source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.over_60.len(), 1);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn ambiguous_project_and_vendor_globs_do_not_fall_back_to_the_prelude() -> TestResult {
    let found = inspect(
        "mod local { macro_rules! format { () => { 1 }; } pub(crate) use format; } use local::*; use std::*; fn run() { format!(); }",
    )?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 1);
    check!(validate_report(&report(found)).is_err());
    Ok(())
}

#[test]
fn unknown_vendor_exports_and_unmeasurable_project_shadows_fail_closed() -> TestResult {
    for source in [
        "use tracing::format; fn run() { format!(\"value\"); }",
        "use restate_sdk::prelude::*; fn run() { restate_sdk::prelude::format!(\"value\"); }",
        "use restate_sdk::prelude::*; macro_rules! vec { ($name:ident) => { fn $name() {} }; } fn run() { vec!(generated); }",
    ] {
        let found = inspect(source)?;
        check!(eq; found.callables, 1);
        check!(validate_report(&report(found)).is_err());
    }
    Ok(())
}

#[test]
fn imported_macros_do_not_shadow_qualified_crate_namespaces() -> TestResult {
    let found = inspect(
        "use anyhow::anyhow; use local::pack as tracing; mod local { macro_rules! pack { ($value:expr) => { $value }; } pub(crate) use pack; } fn run() { anyhow!(\"message\"); anyhow::ensure!(true); tracing!(|| { step(); }); tracing::warn!(\"observed\"); }",
    )?;
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn type_namespace_names_do_not_erase_imported_macro_bindings() -> TestResult {
    let found = inspect("use anyhow::anyhow; struct anyhow; fn run() { anyhow!(\"message\"); }")?;
    check!(eq; found.callables, 1);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn same_named_module_and_function_do_not_shadow_prelude_macros() -> TestResult {
    let found = inspect(
        "mod write { pub fn write() {} } pub use write::write; mod caller { fn run(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, \"{}\", 1) } }",
    )?;
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn value_reexports_never_exempt_same_named_handwritten_macros() -> TestResult {
    let source = format!(
        "mod write {{ pub fn write() {{}} }} pub use write::write; macro_rules! write {{ () => {{ fn generated() {{{}}} }}; }} fn run() {{ write!(); }}",
        "step();".repeat(59)
    );
    let found = inspect(&source)?;
    check!(eq; found.callables, 3);
    check!(eq; found.over_60.len(), 1);
    check!(eq; found.unresolved.len(), 0);
    check!(validate_report(&report(found)).is_err());
    Ok(())
}
