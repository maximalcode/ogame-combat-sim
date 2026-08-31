//! The regression corpus fixture format.
//!
//! A fixture pairs a [`CombatRequest`] with what a real `OGame` battle actually
//! did, and this crate is everything that can be said about one without
//! fighting it: parsing, validation, and comparing a recorded result against a
//! simulated one.
//!
//! It has two consumers and they are why the format lives here rather than in
//! the test that runs it. `combat-core/tests/regression_corpus.rs` walks the
//! corpus in CI; `combat-cli fixture` gives a contributor the same checks
//! before they open a pull request. A second copy of these rules would drift,
//! and the drift would show up as a fixture that passes locally and fails in
//! CI — the worst possible moment to learn the two disagree.
//!
//! Nothing here depends on `combat-core`. [`run_fixture`] takes a closure that
//! produces [`CombatResults`], so validating a fixture never installs the
//! global rayon thread pool that `Simulator::new` would.

use combat_types::names::name_of;
use combat_types::{
    CombatOutcome, CombatRequest, CombatResults, DebrisField, EntityType, FleetComposition,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The only fixture schema this crate understands.
const SCHEMA_VERSION: u8 = 1;

/// A ready-to-edit fixture, as printed by `combat-cli fixture template`.
pub const TEMPLATE: &str = include_str!("template.json");

/// The marker [`TEMPLATE`] leaves in every field a contributor must replace.
///
/// Validation rejects it, so a half-edited template fails `fixture check`
/// rather than reaching review claiming a battle nobody observed.
const PLACEHOLDER_MARKER: &str = "FILL IN";

/// One recorded battle, and the tolerances it is judged by.
///
/// The envelope is `deny_unknown_fields`; the `request` inside it deliberately
/// is not, because it is a plain [`CombatRequest`] and a fixture doubles as a
/// `POST /api/simulate` body. `ignored_request_fields` is what covers the hole
/// that leaves; it runs at parse time and its findings surface through
/// [`Fixture::validation_errors`].
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

    // Not part of the file. [`parse_fixture`] fills this in from the raw JSON,
    // which is the only place both the text and the parsed value exist at once.
    #[serde(skip)]
    ignored_request_fields: Vec<String>,
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

/// What running one fixture came to.
#[derive(Debug, PartialEq, Eq)]
pub enum FixtureStatus {
    Passed,
    /// Blocked on an engine feature; the string says which, and why it matters.
    Skipped(String),
    Failed(Vec<String>),
}

impl Fixture {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The battle to simulate. Handed out so a caller can run it without this
    /// crate needing to know how.
    #[must_use]
    pub fn request(&self) -> &CombatRequest {
        &self.request
    }

    /// Every reason this fixture is not fit to be compared against, or an empty
    /// vector. Checked before simulating, so a malformed fixture fails on its
    /// own terms rather than as a mysterious mismatch.
    #[must_use]
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            errors.push(format!(
                "schema_version is {}, but this harness supports {SCHEMA_VERSION}",
                self.schema_version
            ));
        }

        for (field, value) in self.text_fields() {
            if value.trim().is_empty() {
                errors.push(format!("{field} must not be empty"));
            } else if value.contains(PLACEHOLDER_MARKER) {
                errors.push(format!(
                    "{field} still contains the '{PLACEHOLDER_MARKER}' marker from the template"
                ));
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

        for field in &self.ignored_request_fields {
            errors.push(format!(
                "request.{field} is not a CombatRequest field, so it is silently ignored and the battle is not the one it describes"
            ));
        }

        errors.extend(self.unknown_entity_errors());

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

    /// Entity ids that name no ship or defence.
    ///
    /// The same class of defect as a misspelled request field, and just as
    /// quiet: `FleetComposition` is a map, so `"2014": 30` for `"214": 30` is a
    /// well-formed fixture describing thirty of something that does not exist.
    /// The engine has no stats for it and it simply never fights.
    fn unknown_entity_errors(&self) -> Vec<String> {
        let mut fleets: Vec<(String, &FleetComposition)> = vec![
            (
                "request.attacker.entities".to_string(),
                &self.request.attacker.entities,
            ),
            (
                "request.defender.entities".to_string(),
                &self.request.defender.entities,
            ),
            (
                "observed.attacker_losses".to_string(),
                &self.observed.attacker_losses,
            ),
            (
                "observed.defender_losses".to_string(),
                &self.observed.defender_losses,
            ),
        ];

        // A slot carries a whole party, so a mistyped id hides there too.
        for (field, slots) in [
            ("attacker_slots", &self.request.attacker_slots),
            ("defender_slots", &self.request.defender_slots),
        ] {
            for (index, slot) in slots.iter().flatten().enumerate() {
                fleets.push((
                    format!("request.{field}[{index}].data.entities"),
                    &slot.data.entities,
                ));
            }
        }

        fleets
            .into_iter()
            .flat_map(|(field, fleet)| {
                let mut unknown: Vec<EntityType> = fleet
                    .keys()
                    .copied()
                    .filter(|id| name_of(*id).is_none())
                    .collect();
                unknown.sort_unstable();
                unknown
                    .into_iter()
                    .map(move |id| format!("{field}.{id} is not an entity this simulator knows"))
            })
            .collect()
    }

    /// Every free-text field a human writes, paired with its path in the file.
    /// One list so the empty check and the placeholder check cannot cover
    /// different sets of fields.
    fn text_fields(&self) -> [(&'static str, &str); 6] {
        [
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
        ]
    }

    /// The reason this fixture is not simulated, if it is blocked on a feature
    /// the engine does not model yet. Separate from [`run_fixture`] so the
    /// decision can be tested without a simulator, and so the early return
    /// stays a single readable line.
    #[must_use]
    pub fn skip_reason(&self) -> Option<String> {
        self.blocked_on
            .as_ref()
            .map(|blocker| format!("blocked on '{}': {}", blocker.feature, blocker.reason))
    }

    /// Compare a simulated run against what this fixture recorded.
    ///
    /// Returns every comparison, passing ones included, because the two callers
    /// want different halves: the corpus test reports only failures, and
    /// `combat-cli fixture run` prints the whole table so a contributor can see
    /// the spread their tolerances have to cover.
    #[must_use]
    pub fn evaluate(&self, results: &CombatResults) -> Evaluation {
        let mut numbers = compare_fleet(
            "attacker_losses",
            &self.observed.attacker_losses,
            results,
            |result| &result.attacker_losses,
            &self.tolerance.losses,
        );
        numbers.extend(compare_fleet(
            "defender_losses",
            &self.observed.defender_losses,
            results,
            |result| &result.defender_losses,
            &self.tolerance.losses,
        ));
        numbers.extend(self.compare_debris(results));

        Evaluation {
            outcome: self.compare_outcome(results),
            numbers,
        }
    }

    fn compare_outcome(&self, results: &CombatResults) -> OutcomeCheck {
        let matching = match self.observed.outcome {
            CombatOutcome::AttackersWin => results.attacker_wins,
            CombatOutcome::DefendersWin => results.defender_wins,
            CombatOutcome::Draw => results.draws,
        };

        OutcomeCheck {
            expected: self.observed.outcome.clone(),
            observed_rate: f64::from(matching) / f64::from(results.simulations),
            required_rate: self.tolerance.minimum_observed_outcome_rate,
        }
    }

    fn compare_debris(&self, results: &CombatResults) -> Vec<NumberCheck> {
        let observed = &self.observed.debris;
        let samples = debris_samples(results);

        [
            ("metal", observed.metal, &samples.metal),
            ("crystal", observed.crystal, &samples.crystal),
            ("deuterium", observed.deuterium, &samples.deuterium),
        ]
        .into_iter()
        .map(|(resource, expected, series)| {
            number_check(
                format!("debris.{resource}"),
                expected as f64,
                series,
                results.simulations,
                &self.tolerance.debris,
            )
        })
        .collect()
    }
}

/// One fixture's worth of comparisons, whether or not they passed.
#[derive(Debug)]
pub struct Evaluation {
    pub outcome: OutcomeCheck,
    pub numbers: Vec<NumberCheck>,
}

impl Evaluation {
    /// The failures, phrased for a test report. Empty means the fixture passed.
    #[must_use]
    pub fn failures(&self) -> Vec<String> {
        self.outcome
            .failure()
            .into_iter()
            .chain(self.numbers.iter().filter_map(NumberCheck::failure))
            .collect()
    }
}

/// How often the recorded outcome actually happened.
#[derive(Debug)]
pub struct OutcomeCheck {
    pub expected: CombatOutcome,
    pub observed_rate: f64,
    pub required_rate: f64,
}

impl OutcomeCheck {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.observed_rate >= self.required_rate
    }

    #[must_use]
    pub fn failure(&self) -> Option<String> {
        if self.passed() {
            return None;
        }
        Some(format!(
            "outcome {:?} occurred {:.2}% of runs; required at least {:.2}% (short by {:.2} percentage points)",
            self.expected,
            self.observed_rate * 100.0,
            self.required_rate * 100.0,
            (self.required_rate - self.observed_rate) * 100.0
        ))
    }
}

/// One recorded number against its simulated average.
///
/// `minimum` and `maximum` are across the individual battles rather than
/// derived from `simulated`. They are not used in the pass/fail decision — an
/// average is what a tolerance is written against — but they are the evidence a
/// `justification` needs, and there is nowhere else to get them.
#[derive(Debug)]
pub struct NumberCheck {
    pub label: String,
    pub expected: f64,
    pub simulated: f64,
    pub allowed: f64,
    pub minimum: f64,
    pub maximum: f64,
}

impl NumberCheck {
    #[must_use]
    pub fn difference(&self) -> f64 {
        (self.simulated - self.expected).abs()
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.difference() <= self.allowed
    }

    #[must_use]
    pub fn failure(&self) -> Option<String> {
        if self.passed() {
            return None;
        }
        let (difference, allowed) = (self.difference(), self.allowed);
        Some(format!(
            "{} expected {:.3}, got {:.3}; difference {difference:.3}, allowed {allowed:.3} (exceeds tolerance by {:.3})",
            self.label,
            self.expected,
            self.simulated,
            difference - allowed
        ))
    }
}

/// Every `.json` file under `root`, sorted, so a corpus run is reproducible.
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
    parse_fixture(&contents)
}

/// Parse a fixture from JSON text.
///
/// Private, and [`load_fixture`] is the only caller, deliberately: this is
/// where [`ignored_request_fields`] runs, and a `Fixture` deserialized any
/// other way would report no ignored fields whether or not it had any.
fn parse_fixture(json: &str) -> Result<Fixture, String> {
    let mut fixture: Fixture =
        serde_json::from_str(json).map_err(|error| format!("invalid fixture JSON: {error}"))?;
    fixture.ignored_request_fields = ignored_request_fields(json)?;
    Ok(fixture)
}

/// How far a fixture got: rejected before simulating, skipped, or compared.
///
/// [`FixtureStatus`] is this reduced to pass/fail, which is all the corpus test
/// wants. `combat-cli fixture run` wants the [`Evaluation`] itself, because it
/// prints every comparison rather than only the failing ones — so the order of
/// the checks lives in [`evaluate_fixture`] alone and both callers inherit it.
#[derive(Debug)]
pub enum FixtureRun {
    Invalid(Vec<String>),
    Skipped(String),
    Evaluated(Evaluation),
}

/// Validate, then either skip or simulate and compare.
///
/// `simulate` is a closure rather than a `Simulator` so this crate stays off
/// `combat-core`; see the module docs.
pub fn evaluate_fixture(
    fixture: &Fixture,
    simulate: impl FnOnce(&CombatRequest) -> CombatResults,
) -> FixtureRun {
    let validation_errors = fixture.validation_errors();
    if !validation_errors.is_empty() {
        return FixtureRun::Invalid(validation_errors);
    }

    if let Some(reason) = fixture.skip_reason() {
        return FixtureRun::Skipped(reason);
    }

    FixtureRun::Evaluated(fixture.evaluate(&simulate(&fixture.request)))
}

/// [`evaluate_fixture`] reduced to pass, skip or fail.
pub fn run_fixture(
    fixture: &Fixture,
    simulate: impl FnOnce(&CombatRequest) -> CombatResults,
) -> FixtureStatus {
    match evaluate_fixture(fixture, simulate) {
        FixtureRun::Invalid(errors) => FixtureStatus::Failed(errors),
        FixtureRun::Skipped(reason) => FixtureStatus::Skipped(reason),
        FixtureRun::Evaluated(evaluation) => {
            let discrepancies = evaluation.failures();
            if discrepancies.is_empty() {
                FixtureStatus::Passed
            } else {
                FixtureStatus::Failed(discrepancies)
            }
        }
    }
}

/// Keys inside a fixture's `request` that [`CombatRequest`] threw away.
///
/// The `request` is not `deny_unknown_fields` — it has to stay a body
/// `/api/simulate` accepts — so a misspelled field is dropped in silence, takes
/// its default, and changes the battle without changing the file's validity.
/// That is the sharpest edge in the format, and this is the check that blunts
/// it: parse the request, serialize it back, and report any key that did not
/// survive the round trip.
///
/// The exception is a value serde was going to skip on the way out anyway. Only
/// `skip_serializing_if` does that, and every predicate on `CombatRequest`
/// drops exactly one of null or empty, so a raw value that is null or empty is
/// not evidence the key was misunderstood.
fn ignored_request_fields(json: &str) -> Result<Vec<String>, String> {
    let envelope: Value =
        serde_json::from_str(json).map_err(|error| format!("invalid fixture JSON: {error}"))?;
    let Some(raw_request) = envelope.get("request") else {
        return Ok(Vec::new());
    };

    let parsed: CombatRequest = serde_json::from_value(raw_request.clone())
        .map_err(|error| format!("invalid request in fixture: {error}"))?;
    let understood = serde_json::to_value(&parsed)
        .map_err(|error| format!("could not re-serialize the request: {error}"))?;

    let mut ignored = Vec::new();
    collect_ignored_keys(raw_request, &understood, "", &mut ignored);
    ignored.sort();
    Ok(ignored)
}

/// Field names `CombatRequest` drops on the way out, so their absence from the
/// round trip says nothing about whether they were understood.
///
/// A list rather than a test on the value, because `"universe_setings": null`
/// is a typo and looks exactly like a skipped `Option`. Matched on the leaf key
/// so `lifeform` is covered wherever a party appears, slots included.
/// `fields_serde_skips_on_output_are_not_mistaken_for_typos` is what fails if a
/// new `skip_serializing_if` arrives without being added here.
const FIELDS_SKIPPED_ON_OUTPUT: [&str; 4] = [
    "attacker_slots",
    "defender_slots",
    "lifeform",
    // `PartySlot::name`. Only reachable inside a slot, where nothing else is
    // called `name`.
    "name",
];

fn collect_ignored_keys(raw: &Value, understood: &Value, prefix: &str, ignored: &mut Vec<String>) {
    // Arrays hold structs too — `attacker_slots` is a `Vec<PartySlot>`, and a
    // misspelling inside one defaults just as silently as a top-level one.
    if let (Value::Array(raw_items), Value::Array(understood_items)) = (raw, understood) {
        for (index, (raw_item, understood_item)) in
            raw_items.iter().zip(understood_items).enumerate()
        {
            collect_ignored_keys(
                raw_item,
                understood_item,
                &format!("{prefix}[{index}]"),
                ignored,
            );
        }
        return;
    }

    let (Some(raw_fields), Some(understood_fields)) = (raw.as_object(), understood.as_object())
    else {
        return;
    };

    for (key, raw_value) in raw_fields {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match understood_fields.get(key) {
            Some(understood_value) => {
                collect_ignored_keys(raw_value, understood_value, &path, ignored);
            }
            None if FIELDS_SKIPPED_ON_OUTPUT.contains(&key.as_str()) => {}
            None => ignored.push(path),
        }
    }
}

fn compare_fleet(
    label: &str,
    expected: &FleetComposition,
    results: &CombatResults,
    select: impl Fn(&combat_types::SimulationResult) -> &FleetComposition,
    tolerance: &NumericTolerance,
) -> Vec<NumberCheck> {
    let mut entity_types = BTreeSet::new();
    entity_types.extend(expected.keys().copied());
    for result in &results.results {
        entity_types.extend(select(result).keys().copied());
    }

    entity_types
        .into_iter()
        .map(|entity_type| {
            let samples: Vec<f64> = results
                .results
                .iter()
                .map(|result| f64::from(select(result).get(&entity_type).copied().unwrap_or(0)))
                .collect();
            number_check(
                format!("{label}[{entity_type}]"),
                f64::from(expected.get(&entity_type).copied().unwrap_or(0)),
                &samples,
                results.simulations,
                tolerance,
            )
        })
        .collect()
}

/// Per-battle debris totals, transposed from one field per battle into one
/// series per resource. Collected in a single pass rather than by selecting
/// each resource with a closure: three closures over the same struct are three
/// distinct types, and unifying them means function-pointer casts for no gain.
struct DebrisSamples {
    metal: Vec<f64>,
    crystal: Vec<f64>,
    deuterium: Vec<f64>,
}

fn debris_samples(results: &CombatResults) -> DebrisSamples {
    let mut samples = DebrisSamples {
        metal: Vec::with_capacity(results.results.len()),
        crystal: Vec::with_capacity(results.results.len()),
        deuterium: Vec::with_capacity(results.results.len()),
    };

    for result in &results.results {
        samples.metal.push(result.debris_field.metal as f64);
        samples.crystal.push(result.debris_field.crystal as f64);
        samples.deuterium.push(result.debris_field.deuterium as f64);
    }

    samples
}

fn number_check(
    label: String,
    expected: f64,
    samples: &[f64],
    simulations: u32,
    tolerance: &NumericTolerance,
) -> NumberCheck {
    // Averaged over `simulations` rather than over `samples.len()`: a battle
    // that produced no result should drag the average down, not be excused
    // from it.
    let simulated = samples.iter().sum::<f64>() / f64::from(simulations);

    NumberCheck {
        label,
        expected,
        simulated,
        allowed: tolerance.absolute + expected.abs() * tolerance.relative,
        minimum: samples.iter().copied().fold(f64::INFINITY, f64::min),
        maximum: samples.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
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

#[cfg(test)]
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
        let check = NumberCheck {
            label: "defender_losses[204]".to_string(),
            expected: 100.0,
            simulated: 112.0,
            allowed: 7.0,
            minimum: 100.0,
            maximum: 120.0,
        };

        assert_eq!(
            check.failure(),
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

    #[test]
    fn a_misspelled_request_field_is_reported_rather_than_ignored() {
        let json = fixture_json(&request_json(r#""simulationz": 25"#));

        assert_eq!(
            ignored_request_fields(&json).expect("the fixture should parse"),
            vec!["simulationz".to_string()]
        );
    }

    #[test]
    fn a_misspelling_nested_inside_a_party_is_reported_with_its_path() {
        let json = fixture_json(
            r#"{
                 "attacker": { "technology": { "weapons": 12 }, "entities": {} },
                 "defender": { "technology": {}, "entities": {} },
                 "use_rapid_fire": true,
                 "simulations": 25
               }"#,
        );

        assert_eq!(
            ignored_request_fields(&json).expect("the fixture should parse"),
            vec!["attacker.technology.weapons".to_string()]
        );
    }

    #[test]
    fn entity_ids_are_map_keys_and_are_never_reported_as_unknown() {
        let json = fixture_json(
            r#"{
                 "attacker": { "technology": {}, "entities": { "214": 3, "204": 9 } },
                 "defender": { "technology": {}, "entities": { "401": 50 } },
                 "use_rapid_fire": true,
                 "simulations": 25
               }"#,
        );

        assert_eq!(
            ignored_request_fields(&json).expect("the fixture should parse"),
            Vec::<String>::new()
        );
    }

    // The round-trip check would report a false positive for any field serde
    // drops on the way out. These are every one that exists today; if a new
    // `skip_serializing_if` appears on `CombatRequest` or the types below it,
    // this fails, and the fix is to add the name to FIELDS_SKIPPED_ON_OUTPUT.
    #[test]
    fn fields_serde_skips_on_output_are_not_mistaken_for_typos() {
        let json = fixture_json(
            r#"{
                 "attacker": { "technology": {}, "entities": {}, "lifeform": {} },
                 "defender": { "technology": {}, "entities": {} },
                 "attacker_slots": [
                   { "id": "A1", "name": null, "data": { "technology": {}, "entities": {} } }
                 ],
                 "defender_slots": null,
                 "use_rapid_fire": true,
                 "simulations": 25
               }"#,
        );

        assert_eq!(
            ignored_request_fields(&json).expect("the fixture should parse"),
            Vec::<String>::new()
        );
    }

    // A slot holds a whole `PartyData`, and neither it nor `PartySlot` is
    // `deny_unknown_fields`, so a misspelling inside one defaults just as
    // quietly as a top-level one. The check has to descend through the array to
    // see it.
    #[test]
    fn a_misspelling_inside_a_slot_is_reported_with_its_index() {
        let json = fixture_json(
            r#"{
                 "attacker": { "technology": {}, "entities": {} },
                 "defender": { "technology": {}, "entities": {} },
                 "attacker_slots": [
                   { "id": "A1", "data": { "technology": {}, "entities": {} } },
                   { "id": "A2", "data": { "technology": { "weapons": 12 }, "entities": {} } }
                 ],
                 "use_rapid_fire": true,
                 "simulations": 25
               }"#,
        );

        assert_eq!(
            ignored_request_fields(&json).expect("the fixture should parse"),
            vec!["attacker_slots[1].data.technology.weapons".to_string()]
        );
    }

    // A misspelled key set to null is indistinguishable from a skipped
    // `Option` by value alone, which is why the carve-out is a list of names.
    #[test]
    fn a_misspelled_field_set_to_null_is_still_reported() {
        let json = fixture_json(&request_json(r#""universe_setings": null"#));

        assert_eq!(
            ignored_request_fields(&json).expect("the fixture should parse"),
            vec!["universe_setings".to_string()]
        );
    }

    #[test]
    fn an_entity_id_that_names_no_ship_is_rejected() {
        let json = fixture_json(
            r#"{
                 "attacker": { "technology": {}, "entities": { "2014": 30 } },
                 "defender": { "technology": {}, "entities": { "401": 5 } },
                 "use_rapid_fire": true,
                 "simulations": 25
               }"#,
        );
        let errors = parse_fixture(&json)
            .expect("the fixture should parse")
            .validation_errors();

        assert!(
            errors.contains(
                &"request.attacker.entities.2014 is not an entity this simulator knows".to_string()
            ),
            "expected the unknown entity id to be rejected, got {errors:?}"
        );
    }

    /// A minimal valid request, plus whatever extra keys a test wants to try.
    fn request_json(extra_fields: &str) -> String {
        format!(
            r#"{{
              "attacker": {{ "technology": {{}}, "entities": {{}} }},
              "defender": {{ "technology": {{}}, "entities": {{}} }},
              "use_rapid_fire": true,
              "simulations": 25,
              {extra_fields}
            }}"#
        )
    }

    #[test]
    fn the_shipped_template_is_a_structurally_valid_fixture() {
        let fixture = parse_fixture(TEMPLATE).expect("the template should parse");
        assert!(
            fixture.ignored_request_fields.is_empty(),
            "the template must not teach a field name the engine ignores: {:?}",
            fixture.ignored_request_fields
        );
    }

    #[test]
    fn the_shipped_template_fails_validation_until_it_is_filled_in() {
        let errors = parse_fixture(TEMPLATE)
            .expect("the template should parse")
            .validation_errors();

        assert!(
            errors
                .iter()
                .any(|error| error.contains("provenance.source")
                    && error.contains(PLACEHOLDER_MARKER)),
            "expected an unfilled-placeholder error, got {errors:?}"
        );
    }

    fn fixture_json(request: &str) -> String {
        format!(
            r#"{{
              "schema_version": 1,
              "name": "test",
              "provenance": {{
                "observed_battle": false,
                "source": "test",
                "universe": "test",
                "approximate_date": "test",
                "game_version": "test",
                "publication_consent": false
              }},
              "request": {request},
              "observed": {{
                "outcome": "Draw",
                "attacker_losses": {{}},
                "defender_losses": {{}},
                "debris": {{ "metal": 0, "crystal": 0, "deuterium": 0 }}
              }},
              "tolerance": {{
                "minimum_observed_outcome_rate": 1.0,
                "losses": {{ "absolute": 0.0, "relative": 0.0 }},
                "debris": {{ "absolute": 0.0, "relative": 0.0 }},
                "justification": "test"
              }}
            }}"#
        )
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
            ignored_request_fields: Vec::new(),
        }
    }
}
