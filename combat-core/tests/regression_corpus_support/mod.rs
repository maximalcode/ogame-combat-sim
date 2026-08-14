use combat_core::Simulator;
use combat_types::{CombatOutcome, CombatRequest, CombatResults, DebrisField, FleetComposition};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    schema_version: u8,
    name: String,
    provenance: Provenance,
    #[serde(default)]
    blocked_on: Option<Blocker>,
    request: CombatRequest,
    observed: ObservedResult,
    tolerance: Tolerance,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    observed_battle: bool,
    source: String,
    universe: String,
    approximate_date: String,
    game_version: String,
    publication_consent: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blocker {
    feature: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedResult {
    outcome: CombatOutcome,
    attacker_losses: FleetComposition,
    defender_losses: FleetComposition,
    debris: DebrisField,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tolerance {
    minimum_observed_outcome_rate: f64,
    losses: NumericTolerance,
    debris: NumericTolerance,
    justification: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NumericTolerance {
    absolute: f64,
    relative: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FixtureStatus {
    Passed,
    Skipped(String),
    Failed(Vec<String>),
}

impl Fixture {
    pub fn name(&self) -> &str {
        &self.name
    }

    fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            errors.push(format!(
                "schema_version is {}, but this harness supports {SCHEMA_VERSION}",
                self.schema_version
            ));
        }

        for (field, value) in [
            ("name", self.name.as_str()),
            ("provenance.source", self.provenance.source.as_str()),
            ("provenance.universe", self.provenance.universe.as_str()),
            (
                "provenance.approximate_date",
                self.provenance.approximate_date.as_str(),
            ),
            (
                "provenance.game_version",
                self.provenance.game_version.as_str(),
            ),
            (
                "tolerance.justification",
                self.tolerance.justification.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{field} must not be empty"));
            }
        }

        if self.provenance.observed_battle && !self.provenance.publication_consent {
            errors.push(
                "observed battles require provenance.publication_consent to be true".to_string(),
            );
        }

        if self.request.simulations == 0 {
            errors.push("request.simulations must be greater than zero".to_string());
        }

        errors.extend(rate_error(
            "tolerance.minimum_observed_outcome_rate",
            self.tolerance.minimum_observed_outcome_rate,
        ));
        errors.extend(numeric_tolerance_errors(
            "tolerance.losses",
            &self.tolerance.losses,
        ));
        errors.extend(numeric_tolerance_errors(
            "tolerance.debris",
            &self.tolerance.debris,
        ));

        if let Some(blocker) = &self.blocked_on {
            if blocker.feature.trim().is_empty() {
                errors.push("blocked_on.feature must not be empty".to_string());
            }
            if blocker.reason.trim().is_empty() {
                errors.push("blocked_on.reason must not be empty".to_string());
            }
        }

        errors
    }

    /// The reason this fixture is not simulated, if it is blocked on a feature
    /// the engine does not model yet. Separate from [`run_fixture`] so the
    /// decision can be tested without a simulator, and so the early return
    /// stays a single readable line.
    fn skip_reason(&self) -> Option<String> {
        self.blocked_on
            .as_ref()
            .map(|blocker| format!("blocked on '{}': {}", blocker.feature, blocker.reason))
    }
}

pub fn discover_fixtures(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut fixtures = Vec::new();
    discover_json_files(root, &mut fixtures)?;
    fixtures.sort();
    Ok(fixtures)
}

fn discover_json_files(directory: &Path, fixtures: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "could not read an entry in {}: {error}",
                directory.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;

        if file_type.is_dir() {
            discover_json_files(&path, fixtures)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "json")
        {
            fixtures.push(path);
        }
    }

    Ok(())
}

pub fn load_fixture(path: &Path) -> Result<Fixture, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("could not read fixture: {error}"))?;
    serde_json::from_str(&contents).map_err(|error| format!("invalid fixture JSON: {error}"))
}

pub fn run_fixture(fixture: &Fixture, simulator: &Simulator) -> FixtureStatus {
    let validation_errors = fixture.validation_errors();
    if !validation_errors.is_empty() {
        return FixtureStatus::Failed(validation_errors);
    }

    if let Some(reason) = fixture.skip_reason() {
        return FixtureStatus::Skipped(reason);
    }

    let results = simulator.simulate_multiple(&fixture.request);
    let discrepancies = compare_results(fixture, &results);

    if discrepancies.is_empty() {
        FixtureStatus::Passed
    } else {
        FixtureStatus::Failed(discrepancies)
    }
}

fn compare_results(fixture: &Fixture, results: &CombatResults) -> Vec<String> {
    let mut discrepancies = Vec::new();
    discrepancies.extend(compare_outcome(fixture, results));
    discrepancies.extend(compare_fleet(
        "attacker_losses",
        &fixture.observed.attacker_losses,
        results,
        |result| &result.attacker_losses,
        &fixture.tolerance.losses,
    ));
    discrepancies.extend(compare_fleet(
        "defender_losses",
        &fixture.observed.defender_losses,
        results,
        |result| &result.defender_losses,
        &fixture.tolerance.losses,
    ));
    discrepancies.extend(compare_debris(fixture, results));
    discrepancies
}

fn compare_outcome(fixture: &Fixture, results: &CombatResults) -> Option<String> {
    let matching = match fixture.observed.outcome {
        CombatOutcome::AttackersWin => results.attacker_wins,
        CombatOutcome::DefendersWin => results.defender_wins,
        CombatOutcome::Draw => results.draws,
    };
    let actual_rate = f64::from(matching) / f64::from(results.simulations);
    let minimum_rate = fixture.tolerance.minimum_observed_outcome_rate;

    if actual_rate >= minimum_rate {
        return None;
    }

    Some(format!(
        "outcome {:?} occurred {:.2}% of runs; required at least {:.2}% (short by {:.2} percentage points)",
        fixture.observed.outcome,
        actual_rate * 100.0,
        minimum_rate * 100.0,
        (minimum_rate - actual_rate) * 100.0
    ))
}

fn compare_fleet(
    label: &str,
    expected: &FleetComposition,
    results: &CombatResults,
    select: impl Fn(&combat_types::SimulationResult) -> &FleetComposition,
    tolerance: &NumericTolerance,
) -> Vec<String> {
    let mut entity_types = BTreeSet::new();
    entity_types.extend(expected.keys().copied());
    for result in &results.results {
        entity_types.extend(select(result).keys().copied());
    }

    entity_types
        .into_iter()
        .filter_map(|entity_type| {
            let total: f64 = results
                .results
                .iter()
                .map(|result| f64::from(select(result).get(&entity_type).copied().unwrap_or(0)))
                .sum();
            compare_number(
                &format!("{label}[{entity_type}]"),
                f64::from(expected.get(&entity_type).copied().unwrap_or(0)),
                total / f64::from(results.simulations),
                tolerance,
            )
        })
        .collect()
}

fn compare_debris(fixture: &Fixture, results: &CombatResults) -> Vec<String> {
    let observed = &fixture.observed.debris;
    let simulated = average_debris(results);

    [
        ("metal", observed.metal, simulated.metal),
        ("crystal", observed.crystal, simulated.crystal),
        ("deuterium", observed.deuterium, simulated.deuterium),
    ]
    .into_iter()
    .filter_map(|(resource, expected, actual)| {
        compare_number(
            &format!("debris.{resource}"),
            expected as f64,
            actual,
            &fixture.tolerance.debris,
        )
    })
    .collect()
}

/// A [`DebrisField`] averaged across the run. Averaging first is what lets
/// debris be compared by the same value-and-tolerance path as fleet losses.
struct AverageDebris {
    metal: f64,
    crystal: f64,
    deuterium: f64,
}

fn average_debris(results: &CombatResults) -> AverageDebris {
    let mut totals = AverageDebris {
        metal: 0.0,
        crystal: 0.0,
        deuterium: 0.0,
    };

    for result in &results.results {
        totals.metal += result.debris_field.metal as f64;
        totals.crystal += result.debris_field.crystal as f64;
        totals.deuterium += result.debris_field.deuterium as f64;
    }

    let simulations = f64::from(results.simulations);
    AverageDebris {
        metal: totals.metal / simulations,
        crystal: totals.crystal / simulations,
        deuterium: totals.deuterium / simulations,
    }
}

fn compare_number(
    label: &str,
    expected: f64,
    actual: f64,
    tolerance: &NumericTolerance,
) -> Option<String> {
    let difference = (actual - expected).abs();
    let allowed = tolerance.absolute + expected.abs() * tolerance.relative;

    if difference <= allowed {
        return None;
    }

    Some(format!(
        "{label} expected {expected:.3}, got {actual:.3}; difference {difference:.3}, allowed {allowed:.3} (exceeds tolerance by {:.3})",
        difference - allowed
    ))
}

fn rate_error(field: &str, value: f64) -> Option<String> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        return None;
    }
    Some(format!("{field} must be a finite number from 0.0 to 1.0"))
}

fn numeric_tolerance_errors(field: &str, tolerance: &NumericTolerance) -> Vec<String> {
    let mut errors = Vec::new();
    if !tolerance.absolute.is_finite() || tolerance.absolute < 0.0 {
        errors.push(format!("{field}.absolute must be finite and non-negative"));
    }
    errors.extend(rate_error(&format!("{field}.relative"), tolerance.relative));
    errors
}

// This module is only ever compiled into an integration-test binary, so a
// `#[cfg(test)]` gate around these would always be true.
mod tests {
    use super::*;

    #[test]
    fn a_blocked_fixture_is_skipped_with_its_reason() {
        assert_eq!(
            blocked_fixture().skip_reason(),
            Some("blocked on 'instant-calc': the short-circuit is not implemented yet".to_string())
        );
    }

    #[test]
    fn a_numeric_failure_states_how_far_it_exceeded_tolerance() {
        assert_eq!(
            compare_number(
                "defender_losses[204]",
                100.0,
                112.0,
                &NumericTolerance {
                    absolute: 2.0,
                    relative: 0.05,
                },
            ),
            Some(
                "defender_losses[204] expected 100.000, got 112.000; difference 12.000, allowed 7.000 (exceeds tolerance by 5.000)"
                    .to_string()
            )
        );
    }

    #[test]
    fn an_observed_battle_without_consent_is_rejected() {
        let mut fixture = blocked_fixture();
        fixture.provenance.observed_battle = true;
        fixture.blocked_on = None;

        assert!(
            fixture.validation_errors().iter().any(|error| error
                == "observed battles require provenance.publication_consent to be true")
        );
    }

    fn blocked_fixture() -> Fixture {
        Fixture {
            schema_version: SCHEMA_VERSION,
            name: "blocked example".to_string(),
            provenance: Provenance {
                observed_battle: false,
                source: "test-only placeholder".to_string(),
                universe: "not applicable".to_string(),
                approximate_date: "not applicable".to_string(),
                game_version: "not applicable".to_string(),
                publication_consent: false,
            },
            blocked_on: Some(Blocker {
                feature: "instant-calc".to_string(),
                reason: "the short-circuit is not implemented yet".to_string(),
            }),
            request: CombatRequest::default(),
            observed: ObservedResult {
                outcome: CombatOutcome::Draw,
                attacker_losses: FleetComposition::default(),
                defender_losses: FleetComposition::default(),
                debris: DebrisField::default(),
            },
            tolerance: Tolerance {
                minimum_observed_outcome_rate: 1.0,
                losses: NumericTolerance {
                    absolute: 0.0,
                    relative: 0.0,
                },
                debris: NumericTolerance {
                    absolute: 0.0,
                    relative: 0.0,
                },
                justification: "the runner should never reach these values".to_string(),
            },
        }
    }
}
