//! Universe settings reaching the wreck field.
//!
//! The engine used to store `universe_settings` and read none of it, so a
//! universe with defence debris or deuterium debris switched on produced the
//! same numbers as a standard one. These tests drive the settings through the
//! whole simulator rather than through `calculate_debris` alone, because the
//! bug that mattered was the request field never arriving.

use combat_core::Simulator;
use combat_types::{CombatRequest, DebrisSettings, PartyData, UniverseSettings};
use std::collections::HashMap;

/// A battle the attacker always wins outright, so the losses — and therefore
/// the debris — are the defender's entire position and are the same every run.
/// Deathstars against a handful of gauss cannons: nothing the defence fires can
/// scratch a Deathstar's shield, so `simulations` runs all produce one result.
fn one_sided_battle(settings: Option<UniverseSettings>) -> CombatRequest {
    let mut attacker = HashMap::new();
    attacker.insert(214, 5); // 5 Deathstars

    let mut defender = HashMap::new();
    defender.insert(404, 10); // 10 Gauss cannons: 20000 / 15000 / 2000 each

    CombatRequest {
        attacker: PartyData {
            entities: attacker,
            ..Default::default()
        },
        defender: PartyData {
            entities: defender,
            ..Default::default()
        },
        simulations: 1,
        universe_settings: settings,
        ..Default::default()
    }
}

/// The headline acceptance criterion: fleet and defence percentages are two
/// separate figures, and a universe that sets defence debris gets defence
/// debris.
#[test]
fn defence_debris_appears_when_the_universe_enables_it() {
    let results = Simulator::new().simulate_multiple(&one_sided_battle(Some(UniverseSettings {
        debris_fleet: 30,
        debris_defence: 50,
        ..Default::default()
    })));

    let debris = &results.results[0].debris_field;

    // The attacker loses nothing, so every gram is the defender's ten gauss
    // cannons at the *defence* rate: 10 * 20000 * 0.5 and 10 * 15000 * 0.5.
    assert_eq!(debris.metal, 100_000);
    assert_eq!(debris.crystal, 75_000);
    assert_eq!(debris.deuterium, 0, "deuterium is off in this universe");
}

/// The same battle in a standard universe, where defences leave no debris at
/// all. If one percentage were still being applied to both halves this would
/// match the test above.
#[test]
fn defences_leave_no_debris_in_a_standard_universe() {
    let results = Simulator::new().simulate_multiple(&one_sided_battle(Some(
        UniverseSettings::default(), // debris_defence: 0
    )));

    assert_eq!(results.results[0].debris_field.total(), 0);
}

/// Deuterium in debris fields — the per-universe option from v9.2.0-beta1.
#[test]
fn deuterium_reaches_the_debris_field_when_the_universe_allows_it() {
    let with = one_sided_battle(Some(UniverseSettings {
        debris_defence: 50,
        debris_deuterium: true,
        ..Default::default()
    }));
    let without = one_sided_battle(Some(UniverseSettings {
        debris_defence: 50,
        debris_deuterium: false,
        ..Default::default()
    }));

    let simulator = Simulator::new();
    let with = simulator.simulate_multiple(&with);
    let without = simulator.simulate_multiple(&without);

    // 10 gauss cannons * 2000 deuterium * 50%
    assert_eq!(with.results[0].debris_field.deuterium, 10_000);
    assert_eq!(without.results[0].debris_field.deuterium, 0);

    // Switching the option on adds to the field rather than redistributing it.
    assert_eq!(
        with.results[0].debris_field.metal,
        without.results[0].debris_field.metal
    );
    assert_eq!(
        with.results[0].debris_field.total(),
        without.results[0].debris_field.total() + 10_000
    );
}

/// The compatibility criterion. A request that carries no settings block must
/// produce exactly the field it did before any of this existed: the top-level
/// percentage, ships only, no deuterium.
#[test]
fn a_request_without_universe_settings_is_unchanged() {
    let mut request = one_sided_battle(None);
    request.debris_percentage = 30.0;

    // The defender is all defence, which the old engine scored at zero.
    let defence_only = Simulator::new().simulate_multiple(&request);
    assert_eq!(defence_only.results[0].debris_field.total(), 0);

    // And ships are scored at the top-level percentage.
    let mut ships = HashMap::new();
    ships.insert(211, 10); // 10 Bombers: 50000 / 25000 / 15000
    request.defender.entities = ships;

    let results = Simulator::new().simulate_multiple(&request);
    let debris = &results.results[0].debris_field;
    assert_eq!(debris.metal, 150_000);
    assert_eq!(debris.crystal, 75_000);
    assert_eq!(
        debris.deuterium, 0,
        "deuterium debris must stay off without a universe that enables it"
    );
}

/// The settings survive the trip: a caller can read back which rules the run
/// was scored under instead of re-deriving the precedence themselves.
#[test]
fn the_results_report_the_settings_they_were_scored_under() {
    let results = Simulator::new().simulate_multiple(&one_sided_battle(Some(UniverseSettings {
        debris_fleet: 80,
        debris_defence: 60,
        debris_deuterium: true,
        ..Default::default()
    })));

    assert_eq!(
        results.debris_settings,
        DebrisSettings {
            fleet_percentage: 80,
            defence_percentage: 60,
            deuterium: true,
        }
    );

    // And the fallback path reports what it fell back to.
    let mut fallback = one_sided_battle(None);
    fallback.debris_percentage = 45.0;
    let results = Simulator::new().simulate_multiple(&fallback);
    assert_eq!(
        results.debris_settings,
        DebrisSettings {
            fleet_percentage: 45,
            defence_percentage: 0,
            deuterium: false,
        }
    );
}
