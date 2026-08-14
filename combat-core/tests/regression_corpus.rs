//! Runs every real-world regression fixture in `tests/fixtures`.
//!
//! The format, its validation and the comparison all live in `combat-fixtures`,
//! because `combat-cli fixture` offers a contributor the same checks before
//! they open a pull request. This file is only the part that is specific to
//! running the corpus under libtest.

use combat_core::Simulator;
use combat_fixtures::{FixtureStatus, discover_fixtures, load_fixture, run_fixture};
use std::io::Write;
use std::path::PathBuf;

#[test]
fn every_regression_fixture_matches_its_observed_battle() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let paths = discover_fixtures(&fixture_dir)
        .unwrap_or_else(|error| panic!("could not discover regression fixtures: {error}"));

    assert!(
        !paths.is_empty(),
        "no regression fixtures found in {}",
        fixture_dir.display()
    );

    // The simulator installs one process-wide rayon pool. Reuse one instance
    // for the whole corpus instead of trying to initialise it per fixture.
    let simulator = Simulator::new();
    let mut passed = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();

    for path in paths {
        let fixture = match load_fixture(&path) {
            Ok(fixture) => fixture,
            Err(error) => {
                failures.push(format!("{}: {error}", path.display()));
                continue;
            }
        };

        match run_fixture(&fixture, |request| simulator.simulate_multiple(request)) {
            FixtureStatus::Passed => passed += 1,
            FixtureStatus::Skipped(reason) => {
                skipped += 1;
                if let Err(error) = report_skip(fixture.name(), &reason) {
                    failures.push(format!(
                        "{} ('{}'): could not report skipped fixture: {error}",
                        path.display(),
                        fixture.name()
                    ));
                }
            }
            FixtureStatus::Failed(discrepancies) => {
                failures.extend(discrepancies.into_iter().map(|discrepancy| {
                    format!("{} ('{}'): {discrepancy}", path.display(), fixture.name())
                }));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "regression corpus had {} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );

    // The tally goes the same way the skip records do, and for the same
    // reason: a corpus that skipped everything must not read as a green run.
    write_past_capture(&format!(
        "regression corpus: {passed} passed, {skipped} skipped"
    ))
    .expect("the corpus summary should reach stderr");
}

fn report_skip(name: &str, reason: &str) -> std::io::Result<()> {
    write_past_capture(&format!("SKIP regression fixture '{name}': {reason}"))
}

/// Write around libtest's capture hook. `println!` and `eprintln!` are both
/// swallowed for a *passing* test, which is exactly when a hidden skip does
/// the damage, so this writes to the raw stderr handle instead.
fn write_past_capture(line: &str) -> std::io::Result<()> {
    writeln!(std::io::stderr().lock(), "{line}")
}

#[test]
fn skip_records_remain_visible_under_libtest_capture() {
    const CHILD_PROCESS: &str = "OGAME_REGRESSION_CORPUS_SKIP_OUTPUT_CHILD";
    const NAME: &str = "blocked report";
    const REASON: &str = "blocked on 'instant-calc': not implemented";

    if std::env::var_os(CHILD_PROCESS).is_some() {
        report_skip(NAME, REASON).expect("the child should write its SKIP record");
        return;
    }

    // Running this passing test in a child process gives libtest a chance to
    // capture print-macro output. Raw stderr must still reach the OS pipe.
    let output = std::process::Command::new(
        std::env::current_exe().expect("the current test executable should have a path"),
    )
    .args([
        "--exact",
        "skip_records_remain_visible_under_libtest_capture",
    ])
    .env(CHILD_PROCESS, "1")
    .output()
    .expect("the child test process should run");

    assert!(
        output.status.success(),
        "child test failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("SKIP output should be UTF-8"),
        format!("SKIP regression fixture '{NAME}': {REASON}\n")
    );
}
