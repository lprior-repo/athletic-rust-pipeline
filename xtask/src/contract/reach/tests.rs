use anyhow::Result;
use std::fs;
use std::path::Path;

use super::{declarations, masked_lines, module_dir, resolve, Declaration, Patterns};
use crate::scan::Rules;

fn lines(text: &str) -> Vec<String> {
    text.lines().map(str::to_string).collect()
}

fn found(text: &str) -> Result<Vec<(usize, String, Option<String>)>> {
    let original = lines(text);
    let masked = masked_lines(&original, &Rules::compile()?);
    let patterns = Patterns::compile()?;
    Ok(declarations(&masked, &original, &patterns)
        .into_iter()
        .map(|declaration| (declaration.line, declaration.name, declaration.path))
        .collect())
}

#[test]
fn strings_comments_and_templates_never_declare_a_module() -> Result<()> {
    let text = concat!(
        "fn adapter_module(name: &str) -> String {\n",
        "    format!(\n",
        "        r#\"pub mod map;\n",
        "pub mod parse;\n",
        "\"#\n",
        "    )\n",
        "}\n",
        "const QUOTED: &str = \"mod quoted;\";\n",
        "// mod commented;\n",
        "/* mod blocked; */\n",
        "pub mod real;\n",
    );
    check!(eq; found(text)?, vec![(10, "real".to_string(), None)]);
    Ok(())
}

#[test]
fn a_path_attribute_travels_with_its_declaration_across_lines_and_a_block() -> Result<()> {
    let text = concat!(
        "#[cfg(test)]\n",
        "#[path = \"records_tests/joins.rs\"]\n",
        "mod joins;\n",
        "#[path = \"canonical_json/tests.rs\"] mod tests;\n",
        "mod plain;\n",
    );
    check!(eq;
        found(text)?,
        vec![
            (
                2,
                "joins".to_string(),
                Some("records_tests/joins.rs".to_string())
            ),
            (
                3,
                "tests".to_string(),
                Some("canonical_json/tests.rs".to_string())
            ),
            (4, "plain".to_string(), None),
        ]
    );
    Ok(())
}

#[test]
fn a_module_directory_follows_the_root_and_mod_rs_rules() {
    let root = Path::new("crates/x");
    assert_eq!(module_dir(&root.join("src/lib.rs"), true), root.join("src"));
    assert_eq!(
        module_dir(&root.join("src/spawn.rs"), false),
        root.join("src/spawn")
    );
    assert_eq!(
        module_dir(&root.join("src/spawn/mod.rs"), false),
        root.join("src/spawn")
    );
    assert_eq!(
        module_dir(&root.join("tests/parity.rs"), true),
        root.join("tests")
    );
}

#[test]
fn an_inline_module_directory_holds_its_nested_declarations() -> Result<()> {
    let home = tempfile::tempdir()?;
    let base = home.path();
    fs::create_dir_all(base.join("src/inline"))?;
    fs::write(base.join("src/top.rs"), "")?;
    fs::write(base.join("src/inline/nested.rs"), "")?;
    let root = base.join("src/lib.rs");
    fs::write(&root, "mod top;\nmod inline {\n    mod nested;\n}\n")?;
    let rules = Rules::compile()?;
    let patterns = super::Patterns::compile()?;
    let visited = super::walk::visit(&root, true, &rules, &patterns)?;
    check!(eq;
        visited.pending,
        vec![
            (base.join("src/top.rs"), false),
            (base.join("src/inline/nested.rs"), false),
        ]
    );
    check!(eq; visited.failures, Vec::<String>::new());
    home.close()?;
    Ok(())
}

#[test]
fn a_declaration_resolves_against_the_module_directory_and_a_path_attribute_wins() -> Result<()> {
    let home = tempfile::tempdir()?;
    let base = home.path();
    fs::create_dir_all(base.join("src/spawn"))?;
    fs::create_dir_all(base.join("src/model/canonical_json"))?;
    fs::write(base.join("src/spawn/drain.rs"), "")?;
    fs::write(base.join("src/model/canonical_json/tests.rs"), "")?;
    let declaring = base.join("src/spawn.rs");
    fs::write(&declaring, "")?;
    let declaration = Declaration {
        line: 0,
        name: "drain".to_string(),
        path: None,
    };
    check!(eq;
        resolve(&declaring, &base.join("src/spawn"), &declaration),
        vec![base.join("src/spawn/drain.rs")]
    );
    let attributed = Declaration {
        line: 0,
        name: "tests".to_string(),
        path: Some("canonical_json/tests.rs".to_string()),
    };
    check!(eq;
        resolve(
            &base.join("src/model/canonical_json.rs"),
            &base.join("src/model"),
            &attributed
        ),
        vec![base.join("src/model/canonical_json/tests.rs")]
    );
    fs::create_dir_all(base.join("src/model"))?;
    fs::write(base.join("src/model_tests.rs"), "")?;
    let escaping = Declaration {
        line: 0,
        name: "tests".to_string(),
        path: Some("../model_tests.rs".to_string()),
    };
    check!(eq;
        resolve(
            &base.join("src/model/mod.rs"),
            &base.join("src/model"),
            &escaping
        ),
        vec![base.join("src/model_tests.rs")]
    );
    home.close()?;
    Ok(())
}
