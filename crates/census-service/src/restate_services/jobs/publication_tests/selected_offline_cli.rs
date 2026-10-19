use super::{readback, selected_fixture};
use census_domain::model::ProfileSelectionV1;
use census_report::workbook::publication::{current_workbook, verify_published};
use std::path::{Path, PathBuf};
use std::process::Output;

type TestResult<T = (), E = Box<dyn std::error::Error + Send + Sync>> = Result<T, E>;

struct Smoke {
    binary: PathBuf,
    selection: ProfileSelectionV1,
    public: String,
    candidate: String,
    school: String,
    output: tempfile::TempDir,
}

impl Smoke {
    fn new(controlled: &selected_fixture::Controlled) -> TestResult<Self> {
        let binary = std::env::var_os("CENSUS_SELECTED_CLI_BIN")
            .ok_or("CENSUS_SELECTED_CLI_BIN must name the separately built current CLI")?;
        Ok(Self {
            binary: std::fs::canonicalize(binary)?,
            selection: controlled.selection()?,
            public: controlled.public_id().to_owned(),
            candidate: controlled.candidate_id().to_owned(),
            school: controlled.current_school_id().to_owned(),
            output: tempfile::tempdir()?,
        })
    }

    fn refuse_unenrolled(&self) -> TestResult {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        let store = census_store::Store::open(&root)?;
        let (school, _) = census_domain::model::CanonicalSchool::new(
            census_domain::UsJurisdiction::Colorado,
            "Unenrolled fixture school",
            "Fixture",
            None,
        );
        store.append(census_store::Table::Schools, &school)?;
        let identity = store.store_identity()?;
        let original = store.walk_table(census_store::Table::Schools)?;
        drop(store);
        for command in ["workbook", "run"] {
            check!(!self.invoke(&root, command, "unenrolled")?.status.success());
        }
        let reopened = census_store::Store::open(&root)?;
        check!(eq; reopened.store_identity()?, identity);
        check!(eq; reopened.walk_table(census_store::Table::Schools)?, original);
        Ok(())
    }

    fn invoke(&self, store: &Path, command: &str, generation: &str) -> TestResult<Output> {
        Ok(std::process::Command::new(&self.binary)
            .arg("--store")
            .arg(store)
            .arg(command)
            .args([
                "--school-year",
                "2026",
                "--all-states",
                "--generation",
                generation,
            ])
            .args([
                "--selected-store-identity",
                self.selection.store_identity().as_str(),
            ])
            .args([
                "--selected-profile-id",
                self.selection.profile_id().as_str(),
            ])
            .arg("--out")
            .arg(self.output.path().join(command))
            .env_remove("CI")
            .current_dir(store)
            .output()?)
    }

    fn require_mark(&self, command: &str, mark: &str) -> TestResult<PathBuf> {
        let path = current_workbook(&self.output.path().join(command))?;
        self.check_mark(&path, mark)?;
        Ok(path)
    }

    fn check_mark(&self, path: &Path, mark: &str) -> TestResult {
        check!(eq; verify_published(path)?.workbook.mapped_athletes, 1);
        readback::check_selected_output(path, &self.public, &self.candidate, &self.school, mark)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn successful(output: Output) -> TestResult {
    check!(
        output.status.success(),
        "CLI failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
#[ignore = "requires CENSUS_SELECTED_CLI_BIN from the canonical current-source CLI build"]
fn selected_offline_cli_retains_fifty_then_publishes_fifty_one_in_a_new_generation() -> TestResult {
    let controlled = selected_fixture::install()?;
    let smoke = Smoke::new(&controlled)?;
    smoke.refuse_unenrolled()?;
    let (controlled, original) = selected_fixture::with_stopped(controlled, |store| {
        let mut original = [("workbook", PathBuf::new()), ("run", PathBuf::new())];
        for (command, path) in &mut original {
            successful(smoke.invoke(store, command, "g0")?)?;
            *path = smoke.require_mark(command, "50.00")?;
        }
        Ok(original)
    })?;
    selected_fixture::revoke(&controlled)?;
    let (_controlled, ()) = selected_fixture::with_stopped(controlled, |store| {
        for (command, old) in &original {
            let bytes = std::fs::read(old)?;
            let manifest = old
                .parent()
                .ok_or("old generation missing")?
                .join("manifest.json");
            let manifest_bytes = std::fs::read(&manifest)?;
            check!(!smoke.invoke(store, command, "g0")?.status.success());
            check!(eq; std::fs::read(old)?, bytes);
            check!(eq; std::fs::read(&manifest)?, manifest_bytes);
            successful(smoke.invoke(store, command, "g1")?)?;
            let new = smoke.require_mark(command, "51.00")?;
            check!(ne; &new, old);
            smoke.check_mark(old, "50.00")?;
        }
        Ok(())
    })?;
    if std::env::var("CENSUS_SELECTED_CLI_RETAIN").is_ok_and(|value| value == "1") {
        println!(
            "selected_offline_cli_retained={}",
            smoke.output.keep().display()
        );
    }
    Ok(())
}
