//! Benchmarks for the combat engine.
//!
//! Four measurements, chosen to span the engine's operating range rather than
//! to produce a flattering headline number:
//!
//! - `entity_stats/load` — `load_entity_stats()` alone. It is called once per
//!   simulation today, so knowing what it costs on its own is the prerequisite
//!   for deciding whether that matters.
//! - `battle/small` — one ship type per side. Round overhead with almost no
//!   fleet to iterate.
//! - `battle/medium` — 100 v 1000, rapid fire on. The shape of an ordinary
//!   report.
//! - `battle/large_downscaled` — above `DOWNSCALE_THRESHOLD`, so the
//!   approximation path is what gets timed. Without downscaling this case does
//!   not finish in a benchmark's patience, which is why the path exists.
//!
//! Every case runs the same fixed `SIMULATIONS` count so the numbers are
//! comparable across cases and across commits. Fleet and simulator
//! construction happen outside the timed region; only `simulate_multiple` is
//! measured.

use std::collections::HashMap;
use std::hint::black_box;

use combat_core::Simulator;
use combat_types::entities::load_entity_stats;
use combat_types::{CombatRequest, FleetComposition, PartyData, Technology};
use criterion::{Criterion, criterion_group, criterion_main};

/// Fixed across every battle benchmark. Changing it invalidates comparison
/// with previously recorded numbers, so change it deliberately.
const SIMULATIONS: u32 = 100;

const LIGHT_FIGHTER: u16 = 204;
const CRUISER: u16 = 206;
const ROCKET_LAUNCHER: u16 = 401;

/// Tech 10 across the board — the level most published comparison results use.
fn tech() -> Technology {
    Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    }
}

fn fleet(entries: &[(u16, u32)]) -> FleetComposition {
    let mut f = HashMap::new();
    for &(entity_type, count) in entries {
        f.insert(entity_type, count);
    }
    f
}

fn request(
    attacker: FleetComposition,
    defender: FleetComposition,
    use_rapid_fire: bool,
    enable_downscaling: Option<bool>,
) -> CombatRequest {
    CombatRequest {
        attacker: PartyData {
            technology: tech(),
            entities: attacker,
        },
        defender: PartyData {
            technology: tech(),
            entities: defender,
        },
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire,
        simulations: SIMULATIONS,
        enable_downscaling,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    }
}

fn bench_entity_stats(c: &mut Criterion) {
    c.bench_function("entity_stats/load", |b| {
        b.iter(|| black_box(load_entity_stats()));
    });
}

fn bench_battles(c: &mut Criterion) {
    // One global rayon pool, installed once. Building this inside the timed
    // region would measure `load_entity_stats()` again instead of combat.
    let simulator = Simulator::new();

    let mut group = c.benchmark_group("battle");

    let small = request(
        fleet(&[(CRUISER, 1)]),
        fleet(&[(LIGHT_FIGHTER, 20)]),
        false,
        Some(false),
    );
    group.bench_function("small", |b| {
        b.iter(|| black_box(simulator.simulate_multiple(black_box(&small))));
    });

    let medium = request(
        fleet(&[(CRUISER, 100)]),
        fleet(&[(LIGHT_FIGHTER, 1000)]),
        true,
        Some(false),
    );
    group.bench_function("medium", |b| {
        b.iter(|| black_box(simulator.simulate_multiple(black_box(&medium))));
    });

    // 12M ships a side, comfortably over DOWNSCALE_THRESHOLD (10M). Sample
    // size is dropped because criterion's default 100 samples of a battle this
    // size is minutes of wall clock for no extra statistical value.
    let large = request(
        fleet(&[(CRUISER, 6_000_000), (LIGHT_FIGHTER, 6_000_000)]),
        fleet(&[(LIGHT_FIGHTER, 10_000_000), (ROCKET_LAUNCHER, 2_000_000)]),
        true,
        Some(true),
    );
    group.sample_size(10);
    group.bench_function("large_downscaled", |b| {
        b.iter(|| black_box(simulator.simulate_multiple(black_box(&large))));
    });

    group.finish();
}

criterion_group!(benches, bench_entity_stats, bench_battles);
criterion_main!(benches);
