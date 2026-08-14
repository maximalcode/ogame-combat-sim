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
use combat_fixtures::{
    Evaluation, Fixture, NumberCheck, TEMPLATE, discover_fixtures, load_fixture,
};

pub fn template() -> String {
    TEMPLATE.to_owned()
}

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
        let fixture = match load_and_validate(&path) {
            Ok(fixture) => fixture,
            Err(errors) => {
                report.fail(&path, &errors);
                continue;
            }
        };

        if let Some(reason) = fixture.skip_reason() {
            report.pass(&format!(
                "skip {} ('{}'): {reason}",
                path.display(),
                fixture.name()
            ));
            continue;
        }

        let results = simulator.simulate_multiple(fixture.request());
        let evaluation = fixture.evaluate(&results);
        let table = render_evaluation(&path, &fixture, &evaluation);

        if evaluation.failures().is_empty() {
            report.pass(&table);
        } else {
            report.fail_verbatim(&table);
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
    failed: usize,
}

impl Report {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            passed: 0,
            failed: 0,
        }
    }

    fn pass(&mut self, line: &str) {
        self.passed += 1;
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
    /// one. `subject` completes "all N fixtures ...".
    fn finish(self, subject: &str) -> Result<String, String> {
        let body = format!("{}\n", self.lines.join("\n\n"));
        let total = self.passed + self.failed;
        let counted = plural(total);

        if self.failed == 0 {
            return Ok(format!("{body}\nall {counted} {subject}\n"));
        }
        Err(format!(
            "{} of {counted} did not pass\n\n{body}",
            self.failed
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

// One layout for the header and its rows, so the two cannot drift. A macro
// rather than a const because a format string must be a literal where it is
// used — the same reason render.rs states its tables this way.
macro_rules! metric_row {
    ($out:expr, $($cell:expr),+ $(,)?) => {
        let _ = writeln!($out, "  {:<26} {:>14} {:>14} {:>12} {:>12}  {:<21} {}", $($cell),+);
    };
}

fn render_evaluation(path: &Path, fixture: &Fixture, evaluation: &Evaluation) -> String {
    let mut out = String::new();
    let outcome = &evaluation.outcome;

    let _ = writeln!(out, "{} ('{}')", path.display(), fixture.name());
    let _ = writeln!(
        out,
        "  outcome {:?} in {:.2}% of runs, needs {:.2}%  {}",
        outcome.expected,
        outcome.observed_rate * 100.0,
        outcome.required_rate * 100.0,
        verdict(outcome.passed())
    );

    metric_row!(
        out,
        "metric",
        "observed",
        "simulated",
        "difference",
        "allowed",
        "per-battle range",
        ""
    );

    for check in &evaluation.numbers {
        metric_row!(
            out,
            check.label,
            format!("{:.3}", check.expected),
            format!("{:.3}", check.simulated),
            format!("{:.3}", check.difference()),
            format!("{:.3}", check.allowed),
            range(check),
            verdict(check.passed())
        );
    }

    out
}

/// The spread across individual battles, which is the evidence a
/// `tolerance.justification` is supposed to rest on.
fn range(check: &NumberCheck) -> String {
    if check.minimum.is_finite() && check.maximum.is_finite() {
        format!("{:.0} – {:.0}", check.minimum, check.maximum)
    } else {
        "no battles".to_owned()
    }
}

fn verdict(passed: bool) -> &'static str {
    if passed { "ok" } else { "OVER TOLERANCE" }
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
}
