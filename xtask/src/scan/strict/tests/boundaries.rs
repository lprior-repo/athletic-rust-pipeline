use super::*;

#[test]
fn strict_gate_reports_25_26_advisory_boundary_without_hard_failure() -> TestResult {
    let accepted = inspect(&function(25))?;
    check!(eq; accepted.lines.len(), 0);
    validate_report(&report(accepted))?;
    let rejected = inspect(&function(26))?;
    check!(eq; rejected.lines.len(), 1);
    validate_report(&report(rejected))?;
    Ok(())
}

#[test]
fn strict_gate_counts_receivers_and_generic_parameter_types() -> TestResult {
    let source = "impl X { fn five(&self,a:Map<A,B>,b:fn(A,B)->C,c:(A,B),d:D) {} fn six(&self,a:A,b:B,c:C,d:D,e:E) {} }";
    let found = inspect(source)?;
    check!(eq; found.parameters.len(), 1);
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn nested_generics_function_types_and_trait_declarations_have_actual_arity() -> TestResult {
    let source = "trait X { fn five<T: Fn(A,B)>(a: Map<A,B>,b: fn(A,B)->C,c:(A,B),d:[A;2],e:T); fn six(a:A,b:B,c:C,d:D,e:E,f:F) {} }";
    let found = inspect(source)?;
    check!(eq; found.parameters.len(), 1);
    check!(eq; found.callables, 2);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn closures_have_independent_five_six_parameter_boundaries() -> TestResult {
    let found =
        inspect("fn run() { let five=|a,b,c,d,e| create::<A,B>(a); let six=|a,b,c,d,e,f| f; }")?;
    check!(eq; found.parameters.len(), 1);
    check!(eq; found.callables, 3);
    check!(eq; found.unresolved.len(), 0);
    Ok(())
}

#[test]
fn standalone_closure_has_independent_25_26_line_boundary() -> TestResult {
    let accepted = format!("const F: X = || {{\n{}}};", "step();\n".repeat(23));
    let rejected = format!("const F: X = || {{\n{}}};", "step();\n".repeat(24));
    check!(eq; inspect(&accepted)?.lines.len(), 0);
    check!(eq; inspect(&rejected)?.lines.len(), 1);
    Ok(())
}

#[test]
fn nested_function_is_measured_independently_of_violating_outer() -> TestResult {
    let found = inspect(&format!("fn outer() {{\n{}}}", function(26)))?;
    check!(eq; found.lines.len(), 2);
    check!(eq; found.callables, 2);
    Ok(())
}

#[test]
fn packed_statements_preserve_25_26_boundaries_for_functions_and_closures() -> TestResult {
    for header in ["fn boundary()", "const F: X = ||"] {
        let suffix = if header.starts_with("const") { ";" } else { "" };
        let accepted = format!("{header} {{{}}}{suffix}", "step();".repeat(23));
        let rejected = format!("{header} {{{}}}{suffix}", "step();".repeat(24));
        check!(eq; inspect(&accepted)?.lines.len(), 0);
        check!(eq; inspect(&rejected)?.lines.len(), 1);
    }
    Ok(())
}

#[test]
fn packed_fields_and_match_arms_cannot_evade_budget() -> TestResult {
    let fields = (0..26)
        .map(|index| format!("f{index}: x"))
        .collect::<Vec<_>>()
        .join(",");
    let arms = (0..26)
        .map(|index| format!("{index} => x"))
        .collect::<Vec<_>>()
        .join(",");
    for body in [format!("S {{{fields}}};"), format!("match x {{{arms}}};")] {
        check!(eq; inspect(&format!("fn run() {{{body}}}"))?.lines.len(), 1);
    }
    Ok(())
}

#[test]
fn outer_function_and_real_closure_both_exceed_packed_budget() -> TestResult {
    let source = format!("fn run() {{ let f = || {{{}}}; }}", "step();".repeat(24));
    let found = inspect(&source)?;
    check!(eq; found.callables, 2);
    check!(eq; found.lines.len(), 2);
    Ok(())
}

#[test]
fn source_occupied_and_packed_60_61_boundaries_are_hard_limits() -> TestResult {
    for source in [
        function(60),
        format!("fn run() {{{}}}", "step();".repeat(58)),
    ] {
        let found = inspect(&source)?;
        check!(eq; found.lines.len(), 1);
        check!(eq; found.over_60.len(), 0);
        validate_report(&report(found))?;
    }
    for source in [
        function(61),
        format!("fn run() {{{}}}", "step();".repeat(59)),
    ] {
        let found = inspect(&source)?;
        check!(eq; found.over_60.len(), 1);
        check!(validate_report(&report(found)).is_err());
    }
    Ok(())
}

#[test]
fn methods_and_closures_preserve_packed_60_61_hard_limits() -> TestResult {
    for steps in [58, 59] {
        for source in [
            format!("impl X {{ fn run(&self) {{{}}} }}", "step();".repeat(steps)),
            format!("const F: X = || {{{}}};", "step();".repeat(steps)),
        ] {
            let found = inspect(&source)?;
            check!(eq; found.over_60.len(), usize::from(steps == 59));
            check!(eq; validate_report(&report(found)).is_err(), steps == 59);
        }
    }
    Ok(())
}

#[test]
fn formatted_nested_calls_and_arguments_do_not_inflate_source_line_cap() -> TestResult {
    let source = format!(
        "fn run() {{\n{}}}",
        "    consume(convert(x), wrap(y).finish(z), a, b, c);\n".repeat(57)
    );
    let found = inspect(&source)?;
    check!(eq; found.callables, 1);
    check!(eq; found.over_60.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn ordinary_call_arguments_are_not_logical_statement_boundaries() -> TestResult {
    let arguments = std::iter::repeat_n("wrap(x).finish(y)", 80)
        .collect::<Vec<_>>()
        .join(",");
    let found = inspect(&format!("fn run() {{consume({arguments});}}"))?;
    check!(eq; found.lines.len(), 0);
    check!(eq; found.over_60.len(), 0);
    Ok(())
}

#[test]
fn packed_field_and_arm_60_61_boundaries_preserve_the_hard_cap() -> TestResult {
    for count in [57, 58] {
        let fields = (0..count)
            .map(|index| format!("f{index}: x"))
            .collect::<Vec<_>>()
            .join(",");
        let arms = (0..count)
            .map(|index| format!("{index} => x"))
            .collect::<Vec<_>>()
            .join(",");
        for body in [format!("S {{{fields}}};"), format!("match x {{{arms}}};")] {
            let found = inspect(&format!("fn run() {{{body}}}"))?;
            check!(eq; found.over_60.len(), usize::from(count == 58));
        }
    }
    Ok(())
}

#[test]
fn formatted_nested_closures_do_not_inflate_the_enclosing_callable() -> TestResult {
    let statements = "    consume(|| convert(x), || wrap(y).finish(z));\n".repeat(57);
    let found = inspect(&format!("fn run() {{\n{statements}}}"))?;
    check!(eq; found.callables, 115);
    check!(eq; found.over_60.len(), 0);
    validate_report(&report(found))?;
    Ok(())
}

#[test]
fn comma_less_select_handlers_measure_handwritten_closures() -> TestResult {
    let source = "fn run() { tokio::select! { () = first() => {} () = second() => { let nested = || step(); } else => { done(); } } }";
    let found = inspect(source)?;
    check!(eq; found.unresolved.len(), 0);
    check!(eq; found.callables, 2);
    validate_report(&report(found))?;
    Ok(())
}
