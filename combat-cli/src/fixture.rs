//! The `fixture` subcommand: author, check and try out regression fixtures.
//!
//! The corpus is the only claim this simulator makes that anyone can check, and
//! it is filled by hand from real `OGame` reports. That makes the authoring
//! experience the thing standing between a report someone has and a fixture in
//! the repository, so all three commands here exist to remove one specific
//! obstacle:
//!
//! - `template` — nobody should retype a 60-line envelope from the README.
//! - `check` — the mistakes should surface before review, not in CI. In
//!   particular a misspelled `request` field, which is silently ignored by
//!   design and quietly changes the battle.
//! - `run` — a `tolerance.justification` is mandatory and cannot honestly be
//!   written without seeing how much the result actually moves between battles.
//!
//! Every check here is `combat-fixtures`' own, which is also what the corpus
//! test runs. A second implementation would drift, and the drift would show up
//! as a fixture that passes locally and fails in CI.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use combat_core::Simulator;
use combat_fixtures::{Fixture, FixtureRun, discover_fixtures, evaluate_fixture, load_fixture};

use crate::render::render_evaluation;

pub fn check(paths: &[PathBuf]) -> Result<String, String> {
    let mut report = Report::new();

    for path in expand(paths)? {
        match load_and_validate(&path) {
            Ok(fixture) => report.pass(&format!("ok   {} ('{}')", path.display(), fixture.name())),
            Err(errors) => report.fail(&path, &errors),
        }
    }

    report.finish("valid")
}

pub fn run(paths: &[PathBuf]) -> Result<String, String> {
    let paths = expand(paths)?;

    // One `Simulator` for the whole command: `new` installs a process-wide
    // rayon pool, and a second call silently does nothing.
    let simulator = Simulator::new();
    let mut report = Report::new();

    for path in paths {
        let fixture = match load_fixture(&path) {
            Ok(fixture) => fixture,
            Err(error) => {
                report.fail(&path, &[error]);
                continue;
            }
        };

        // Through `evaluate_fixture` rather than by validating, skipping and
        // comparing here: the corpus test goes the same way, and the order of
        // those steps should exist in one place.
        match evaluate_fixture(&fixture, |request| simulator.simulate_multiple(request)) {
            FixtureRun::Invalid(errors) => report.fail(&path, &errors),
            FixtureRun::Skipped(reason) => report.skip(&format!(
                "SKIP {} ('{}'): {reason}",
                path.display(),
                fixture.name()
            )),
            FixtureRun::Evaluated(evaluation) => {
                let table = render_evaluation(&path, fixture.name(), &evaluation);
                if evaluation.failures().is_empty() {
                    report.pass(&table);
                } else {
                    report.fail_verbatim(&table);
                }
            }
        }
    }

    report.finish("matched their recorded battle")
}

/// Load a fixture and apply every validation rule the corpus test applies.
fn load_and_validate(path: &Path) -> Result<Fixture, Vec<String>> {
    let fixture = load_fixture(path).map_err(|error| vec![error])?;
    let errors = fixture.validation_errors();
    if errors.is_empty() {
        Ok(fixture)
    } else {
        Err(errors)
    }
}

/// Turn the paths given on the command line into fixture files.
///
/// A directory is walked with the corpus test's own discovery, so pointing this
/// at `combat-core/tests/fixtures` checks exactly the set CI will check.
fn expand(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    for path in paths {
        if path.is_dir() {
            files.extend(discover_fixtures(path)?);
        } else {
            files.push(path.clone());
        }
    }

    if files.is_empty() {
        return Err("no fixture files were found in the given paths".to_owned());
    }

    Ok(files)
}

/// Accumulates per-fixture output and decides the exit status.
///
/// Failures are collected rather than returned at the first one: someone
/// checking a directory wants every problem in one pass, not one per run.
struct Report {
    lines: Vec<String>,
    passed: usize,
    skipped: usize,
    failed: usize,
}

impl Report {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            passed: 0,
            skipped: 0,
            failed: 0,
        }
    }

    fn pass(&mut self, line: &str) {
        self.passed += 1;
        self.lines.push(line.to_owned());
    }

    /// Counted apart from a pass, and stated in the summary, for the reason the
    /// corpus test writes its skip records past libtest's capture: a fixture
    /// that was never compared has not agreed with anything, and a run that
    /// skipped everything must not read as a clean one.
    fn skip(&mut self, line: &str) {
        self.skipped += 1;
        self.lines.push(line.to_owned());
    }

    fn fail(&mut self, path: &Path, errors: &[String]) {
        let mut block = format!("FAIL {}", path.display());
        for error in errors {
            let _ = write!(block, "\n       - {error}");
        }
        self.fail_verbatim(&block);
    }

    fn fail_verbatim(&mut self, block: &str) {
        self.failed += 1;
        self.lines.push(block.to_owned());
    }

    /// `Ok` prints to stdout and exits zero; `Err` prints to stderr and exits
    /// one. `subject` completes "N fixtures ...".
    fn finish(self, subject: &str) -> Result<String, String> {
        let body = format!("{}\n", self.lines.join("\n\n"));
        let total = self.passed + self.skipped + self.failed;
        let skipped = match self.skipped {
            0 => String::new(),
            count => format!(", {count} skipped"),
        };

        if self.failed == 0 {
            return Ok(format!(
                "{body}\n{} {subject}{skipped}\n",
                plural(self.passed)
            ));
        }
        Err(format!(
            "{} of {} did not pass{skipped}\n\n{body}",
            self.failed,
            plural(total)
        ))
    }
}

fn plural(count: usize) -> String {
    if count == 1 {
        "1 fixture".to_owned()
    } else {
        format!("{count} fixtures")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The corpus directory the regression test runs, reached from this crate.
    fn corpus() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("combat-core")
            .join("tests")
            .join("fixtures")
    }

    #[test]
    fn a_directory_expands_to_the_fixtures_inside_it() {
        let files = expand(&[corpus()]).expect("the corpus directory should expand");

        assert!(
            !files.is_empty(),
            "the shipped corpus should contain at least one fixture"
        );
        assert!(
            files
                .iter()
                .all(|path| path.extension().is_some_and(|ext| ext == "json")),
            "expand should only yield .json files, got {files:?}"
        );
    }

    // `check` and the corpus test apply the same rules from `combat-fixtures`.
    // This is the assertion that says so: if they ever diverge, a contributor
    // gets a green `fixture check` and a red CI run, which is the one failure
    // this whole arrangement exists to prevent.
    #[test]
    fn the_shipped_corpus_passes_the_checks_the_cli_applies() {
        assert!(
            check(&[corpus()]).is_ok(),
            "the shipped corpus failed `fixture check`: {:?}",
            check(&[corpus()])
        );
    }

    #[test]
    fn expand_says_so_when_nothing_matched() {
        assert_eq!(
            expand(&[]),
            Err("no fixture files were found in the given paths".to_owned())
        );
    }

    #[test]
    fn a_failing_run_reports_every_problem_rather_than_the_first() {
        let mut report = Report::new();
        report.pass("ok   one.json");
        report.fail(
            Path::new("two.json"),
            &["first problem".to_owned(), "second problem".to_owned()],
        );

        let error = report.finish("valid").expect_err("a failure should be Err");

        assert!(error.starts_with("1 of 2 fixtures did not pass"), "{error}");
        assert!(error.contains("- first problem"), "{error}");
        assert!(error.contains("- second problem"), "{error}");
    }

    #[test]
    fn one_fixture_is_not_reported_as_1_fixtures() {
        assert_eq!(plural(1), "1 fixture");
        assert_eq!(plural(2), "2 fixtures");
    }

    // A skipped fixture was never compared against anything, so a run that
    // skipped one must not summarise as though it had. The corpus test goes to
    // the trouble of writing its skip records past libtest's capture for the
    // same reason.
    #[test]
    fn a_skipped_fixture_is_not_counted_as_one_that_matched() {
        let mut report = Report::new();
        report.pass("ok   one.json");
        report.skip("SKIP two.json ('blocked'): blocked on 'instant-calc'");

        let summary = report
            .finish("matched their recorded battle")
            .expect("a skip is not a failure");

        assert!(
            summary.ends_with("1 fixture matched their recorded battle, 1 skipped\n"),
            "{summary}"
        );
    }
}
