//! v13's instant calculation, asserted where it is visible: through a whole
//! battle rather than against the predicate that decides it.
//!
//! The predicate has its own unit tests next to it in `combat-core/src/
//! instant.rs`, and they are about the conditions. These are about the two
//! promises the rule makes to everything downstream of it — that a battle it
//! decides comes out where the six rounds would have put it, and that the
//! result is the same shape as any other battle's — plus the half of the
//! changelog that turned out to be nothing to do: espionage probes never lost
//! automatically here and still do not.

use approx::assert_relative_eq;
use combat_core::{Combat, ReportBuilder, Simulator};
use combat_types::{
    CombatOutcome, CombatRequest, EntityType, LifeformBonus, LifeformBonuses, PartyData, PartySlot,
    PlanetResources, Technology,
};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

const LIGHT_FIGHTER: EntityType = 204;
const CRUISER: EntityType = 206;
const BATTLESHIP: EntityType = 207;
const ESPIONAGE_PROBE: EntityType = 210;
const DESTROYER: EntityType = 213;
const DEATHSTAR: EntityType = 214;
const ROCKET_LAUNCHER: EntityType = 401;
const PLASMA_TURRET: EntityType = 406;

/// A side of plain ships at Weapons/Shielding/Armour 0.
fn fleet(units: &[(EntityType, u32)]) -> PartyData {
    PartyData {
        entities: units.iter().copied().collect(),
        ..Default::default()
    }
}

/// One battle down each path, from the same starting position and the same
/// seed. The instant path never touches the RNG; seeding both the same way
/// keeps the comparison honest anyway, because then the only difference between
/// the two runs is the rule under test.
///
/// Both rapid-fire settings, every time. Rapid fire is the difference between a
/// Deathstar firing once a round and firing a thousand times, so a rule checked
/// only with it on is a rule checked on half the requests the API accepts — and
/// the half it was not checked on is where it was wrong.
///
/// Asserts everything a caller can read out of a battle except the round
/// figures, which are the one thing the two paths are supposed to disagree
/// about: a battle that was never fought reports 0 rounds and no round detail.
fn both_paths_agree(attacker: &PartyData, defender: &PartyData) {
    for use_rapid_fire in [false, true] {
        let combat = Combat::new();

        let mut instant_rng = SmallRng::seed_from_u64(20_260_720);
        let instant =
            combat.simulate_single(attacker, defender, use_rapid_fire, false, &mut instant_rng);

        let mut rounds_rng = SmallRng::seed_from_u64(20_260_720);
        let fought = combat.simulate_single_through_the_rounds(
            attacker,
            defender,
            use_rapid_fire,
            false,
            &mut rounds_rng,
        );

        assert_eq!(
            instant.rounds, 0,
            "this battle was supposed to be instantly calculated, and it fought {} rounds",
            instant.rounds
        );
        assert!(
            fought.rounds > 0,
            "the comparison is worthless unless the other path actually fought the battle"
        );
        assert!(
            instant
                .round_details
                .as_deref()
                .is_some_and(<[_]>::is_empty),
            "a battle with no rounds in it has no round detail to report"
        );

        assert!(
            instant.outcome == fought.outcome,
            "rapid fire {use_rapid_fire}: the two paths disagree about who won"
        );
        assert_eq!(instant.attacker_losses, fought.attacker_losses);
        assert_eq!(instant.defender_losses, fought.defender_losses);
        assert_eq!(instant.attacker_remaining, fought.attacker_remaining);
        assert_eq!(instant.defender_remaining, fought.defender_remaining);
    }
}

/// The load-bearing claim: the short-circuit is an optimisation, not a second
/// set of rules. Every battle here satisfies the rule, is decided by it, and is
/// then fought anyway to see whether the rule was telling the truth.
#[test]
fn an_instantly_calculated_battle_matches_the_one_it_replaced() {
    // The changelog's own case: a fleet against nothing but espionage probes.
    both_paths_agree(
        &fleet(&[(BATTLESHIP, 20_000)]),
        &fleet(&[(ESPIONAGE_PROBE, 1)]),
    );

    // The same, with enough probes that they are a fleet rather than a rounding
    // error, and with the attacker's own probes along for the ride — which are
    // hitpoints the rule counts and shots it does not.
    both_paths_agree(
        &fleet(&[(DESTROYER, 100_000), (ESPIONAGE_PROBE, 40)]),
        &fleet(&[(ESPIONAGE_PROBE, 30)]),
    );

    // A defender can be the overwhelming side. Three probes arriving over ten
    // thousand Plasma Turrets is the rule read the other way round.
    both_paths_agree(
        &fleet(&[(ESPIONAGE_PROBE, 3)]),
        &fleet(&[(PLASMA_TURRET, 10_000)]),
    );

    // Solar Satellites are armed, barely, so the losing side's attack power is
    // not zero here — the ratio is doing the work rather than a division by
    // nothing.
    both_paths_agree(
        &fleet(&[(DESTROYER, 200_000)]),
        &fleet(&[(212, 50), (ESPIONAGE_PROBE, 10)]),
    );
}

/// The rule reads effective attack power, so a level of Weapons can be what
/// decides whether a battle is fought at all. Two identical fleets, one
/// researched: only the researched one is over the line, and both agree on the
/// battle.
#[test]
fn the_rule_reads_the_firepower_the_battle_would_be_fought_at() {
    let fighters = |weapons: u8, lifeform: LifeformBonuses| PartyData {
        technology: Technology {
            weapon: weapons,
            ..Default::default()
        },
        entities: HashMap::from([(LIGHT_FIGHTER, 15_000)]),
        lifeform,
    };
    let probe = fleet(&[(ESPIONAGE_PROBE, 1)]);
    let rounds_fought = |attacker: &PartyData| {
        Simulator::new()
            .simulate_multiple(&CombatRequest {
                attacker: attacker.clone(),
                defender: probe.clone(),
                simulations: 1,
                ..Default::default()
            })
            .results[0]
            .rounds
    };

    // 15,000 Light Fighters are worth 750,000 at Weapons 0, which is under the
    // margin the wipe has to clear against a probe's hull.
    assert!(rounds_fought(&fighters(0, LifeformBonuses::default())) > 0);
    // At Weapons 10 the same fleet is worth 1,500,000 and is over it.
    assert_eq!(rounds_fought(&fighters(10, LifeformBonuses::default())), 0);
    // A lifeform bonus is the other kind of stat modifier and reaches the rule
    // by the other seam. +100% on Light Fighters is worth the same ten levels.
    let bonus = LifeformBonuses::from_iter([(LIGHT_FIGHTER, LifeformBonus::uniform(100.0))]);
    assert_eq!(rounds_fought(&fighters(0, bonus)), 0);

    // ...and none of it changed the battle, only how long it took to say so.
    both_paths_agree(&fighters(10, LifeformBonuses::default()), &probe);
}

/// Nothing in this engine ever made a probe-only fleet lose on sight, which is
/// the half of the v13 change that was already true here. This is what stops it
/// being introduced by accident — including by the new rule, which sees a
/// hundred probes with no attack power at all and still declines to hand the
/// battle to a single Rocket Launcher that cannot get through them in six
/// rounds.
#[test]
fn a_probe_only_fleet_is_not_an_automatic_loss() {
    let results = Simulator::new().simulate_multiple(&CombatRequest {
        attacker: fleet(&[(ESPIONAGE_PROBE, 100)]),
        defender: fleet(&[(ROCKET_LAUNCHER, 1)]),
        simulations: 20,
        ..Default::default()
    });

    assert_eq!(
        results.defender_wins, 0,
        "a fleet of probes did not lose the battle, it merely could not win it"
    );
    for result in &results.results {
        assert!(result.rounds > 0, "the battle was fought");
        assert_eq!(result.outcome, CombatOutcome::Draw);
        // One launcher firing once a round can take at most six probes with it.
        let survivors: u32 = result.attacker_remaining.values().sum();
        assert!(survivors >= 94, "{survivors} probes came home");
    }
}

/// A battle the rule decides still has to produce everything the simulator
/// derives from a battle — debris, loot, profit and a report — in the shape a
/// fought battle produces it in. It does, because the short-circuit stops at
/// emptying the losing party and every step after it is the ordinary one.
#[test]
fn an_instantly_calculated_battle_reports_like_any_other() {
    let request = CombatRequest {
        attacker: fleet(&[(CRUISER, 60_000)]),
        defender: fleet(&[(ESPIONAGE_PROBE, 20)]),
        planet_resources: Some(PlanetResources {
            metal: 5_000_000,
            crystal: 2_000_000,
            deuterium: 1_000_000,
        }),
        simulations: 1,
        ..Default::default()
    };

    let results = Simulator::new().simulate_multiple(&request);
    let result = &results.results[0];

    assert_eq!(result.rounds, 0, "no rounds were fought");
    assert_relative_eq!(results.average_rounds, 0.0);
    assert_eq!(result.outcome, CombatOutcome::AttackersWin);
    assert_eq!(
        result.defender_losses,
        HashMap::from([(ESPIONAGE_PROBE, 20)])
    );
    assert!(result.attacker_losses.is_empty());

    // A probe costs 1000 crystal and 30% of it stays in orbit, so twenty of
    // them are a debris field whether or not anyone watched them die.
    assert_eq!(result.debris_field.crystal, 6_000);
    assert!(
        result.loot.metal > 0,
        "sixty thousand Cruisers came to loot"
    );
    assert!(result.attacker_profit > 0);
    assert!(result.defender_profit < 0);

    let report = ReportBuilder::new().build_summary_report(&request, &results);
    assert_eq!(report.rounds, 0);
    assert_eq!(report.outcome, CombatOutcome::AttackersWin);
    assert!(report.defender_fleet_end.ships.is_empty());
    assert_eq!(
        report.attacker_fleet_end.ships,
        HashMap::from([(CRUISER, 60_000)])
    );
    assert!(report.economics.harvest_info.is_some());
}

/// Slot mode takes the same rule, because combined attack power is a property
/// of a side and a slot is only how that side's fleet is reported. The per-slot
/// breakdown comes out of the same code that builds it for a fought battle.
#[test]
fn slots_take_the_same_rule() {
    let slot = |id: &str, units: &[(EntityType, u32)]| PartySlot {
        id: id.to_owned(),
        name: None,
        data: fleet(units),
    };

    let results = Simulator::new().simulate_multiple(&CombatRequest {
        attacker: fleet(&[(BATTLESHIP, 30_000), (CRUISER, 10_000)]),
        defender: fleet(&[(ESPIONAGE_PROBE, 20)]),
        attacker_slots: Some(vec![
            slot("A1", &[(BATTLESHIP, 30_000)]),
            slot("A2", &[(CRUISER, 10_000)]),
        ]),
        defender_slots: Some(vec![slot("D1", &[(ESPIONAGE_PROBE, 20)])]),
        simulations: 1,
        ..Default::default()
    });
    let result = &results.results[0];

    assert_eq!(result.rounds, 0);
    assert_eq!(result.outcome, CombatOutcome::AttackersWin);

    let attacker_slots = result.attacker_slots.as_ref().expect("slot results");
    assert_eq!(attacker_slots.len(), 2);
    for slot in attacker_slots {
        assert!(slot.losses.is_empty(), "{} lost nothing", slot.slot_id);
        assert_eq!(slot.remaining, slot.initial);
    }

    let defender_slots = result.defender_slots.as_ref().expect("slot results");
    assert_eq!(defender_slots.len(), 1);
    assert_eq!(
        defender_slots[0].losses,
        HashMap::from([(ESPIONAGE_PROBE, 20)])
    );
    assert!(defender_slots[0].remaining.is_empty());
}

// Everything from here to `no_battle_the_rule_decides_disagrees_with_the_battle
// _it_replaces` is that test's generator, kept out of its body only because a
// hundred lines is clippy's limit and the interesting part is the loop.

/// The boundary probe bisects, so it fights one seed's battle about fifteen
/// times over. These two keep that affordable: the swarm it aims at and the pack
/// it aims with are both an order of magnitude under the test's own
/// `UNIT_BUDGET` of 30,000, which is plenty of room for the shape it is looking
/// for and not enough to double the suite's runtime.
const PROBE_SWARM_LIMIT: u32 = 3_000;
const PROBE_PACK_LIMIT: u32 = 5_000;

/// A technology level, drawn from either end of the range the type allows.
/// Half the draws stay where an account plausibly is; the other half go to
/// the top, because that is where the shot budget binds and the damage
/// budget does not.
fn tech(rng: &mut SmallRng) -> u8 {
    if rng.random_bool(0.5) {
        rng.random_range(0..=25)
    } else {
        rng.random_range(150..=255)
    }
}

/// One to three types out of `types`, in counts spread over `scale` orders
/// of magnitude, at levels `tech` picked.
fn party(rng: &mut SmallRng, types: &[EntityType], scale: u32) -> PartyData {
    PartyData {
        technology: Technology {
            weapon: tech(rng),
            shield: tech(rng),
            armour: tech(rng),
            ..Default::default()
        },
        entities: (0..rng.random_range(1..=3))
            .map(|_| {
                let entity_type = types[rng.random_range(0..types.len())];
                let count = 10u32.pow(rng.random_range(0..=scale)) * rng.random_range(1..=9);
                (entity_type, count)
            })
            .collect(),
        ..Default::default()
    }
}

/// The smallest pack of Deathstars the rule will still speak for against
/// `defender`, if it will speak for any pack this side of
/// `PROBE_PACK_LIMIT`.
///
/// Bisection is well defined because every condition the rule applies is
/// monotone in the winner's unit count: more hulls is more attack power and
/// more shots, and leaves every shield and every shot size where it was.
/// "The rule spoke" is read as `rounds == 0`, which is what the caller has
/// — both sides are non-empty here, so there is no other way to reach it.
fn smallest_pack_the_rule_speaks_for(
    combat: &Combat,
    pack: &PartyData,
    defender: &PartyData,
    use_rapid_fire: bool,
    seed: u64,
) -> Option<u32> {
    let fires = |count: u32| {
        let mut candidate = pack.clone();
        candidate.entities = HashMap::from([(DEATHSTAR, count)]);
        let mut probe_rng = SmallRng::seed_from_u64(seed);
        combat
            .simulate_single(&candidate, defender, use_rapid_fire, false, &mut probe_rng)
            .rounds
            == 0
    };

    if !fires(PROBE_PACK_LIMIT) {
        return None;
    }
    let (mut smallest, mut largest) = (1, PROBE_PACK_LIMIT);
    while smallest < largest {
        let middle = smallest + (largest - smallest) / 2;
        if fires(middle) {
            largest = middle;
        } else {
            smallest = middle + 1;
        }
    }
    Some(smallest)
}

/// The same claim as `an_instantly_calculated_battle_matches_the_one_it_replaced`,
/// asked of battles nobody chose.
///
/// Hand-picked battles prove the rule works where it was aimed; they cannot
/// prove it is never aimed somewhere it does not. So this walks a fixed set of
/// seeds, builds fleets out of the whole stat table at technology levels it
/// picks itself, fights them with rapid fire on and off, keeps the ones the
/// rule decides to short-circuit, and fights those anyway. The seeds are fixed
/// rather than random because a property test that fails on Tuesdays is not a
/// test; if it ever does fail, the seed in the message rebuilds the battle
/// exactly.
///
/// Three things about the generator are here because the rule was once wrong
/// where this test could not look. Technology reaches the top of the range `u8`
/// allows rather than the range an account plausibly has, because the rule reads
/// *effective* weapon power and Weapons 255 multiplies the stat table by 26.5 —
/// which is where a side's damage budget and its shot budget come apart. Rapid
/// fire is switched by seed rather than pinned on, because a Deathstar with
/// rapid fire 250 against probes fires hundreds of shots a round and without
/// it fires one, and `use_rapid_fire` is a request field with a CLI flag behind
/// it. And one seed in forty is a boundary probe rather than a roll: it bisects
/// for the smallest fleet the rule will still speak for and compares *there*,
/// because a rule that is unsound is unsound at the line it draws and rolling
/// two fleet sizes independently lands on that line about once in ten thousand
/// seeds.
///
/// `TRIGGERS_EXPECTED` is the other half of it. A change that quietly stopped
/// the rule from firing would leave every comparison below trivially true, and
/// a green run would mean nothing at all.
#[test]
fn no_battle_the_rule_decides_disagrees_with_the_battle_it_replaces() {
    /// Fleets big enough to be interesting and small enough that fighting a
    /// few hundred of them stays inside a test suite's patience.
    const UNIT_BUDGET: u32 = 30_000;
    /// A floor under what the seeds below trigger today (66), not a target: it
    /// is here to fail loudly if the rule stops firing, not to pin a count.
    const TRIGGERS_EXPECTED: u32 = 50;
    let entity_db = combat_types::entities::entity_stats();
    let mut types: Vec<EntityType> = entity_db.keys().copied().collect();
    // Missiles never take part in a battle, so a fleet of them is not one.
    types.retain(|entity_type| !matches!(*entity_type, 502 | 503));
    types.sort_unstable();

    // The frail, near-harmless units the rule exists for. Three defenders in
    // four are drawn from these, because a defender rolled uniformly out of the
    // whole table is hardly ever the side of a ten-thousand-fold mismatch.
    let frail = [ESPIONAGE_PROBE, 212, 202, LIGHT_FIGHTER, ROCKET_LAUNCHER];

    let combat = Combat::new();
    let mut triggered = 0;

    for seed in 0..10_000u64 {
        let use_rapid_fire = seed % 2 == 0;
        let mut rng = SmallRng::seed_from_u64(seed);

        let mut defender = party(&mut rng, &types, 1);
        if seed % 4 != 0 {
            // Orders of magnitude rather than a flat range, and both ends earn
            // their place. A handful of frail units is where the rule still
            // fires, so it is where most of the comparisons come from; a
            // thousand of them is what the aimed seeds below need to aim at.
            defender.entities = HashMap::from([(
                frail[rng.random_range(0..frail.len())],
                10u32.pow(rng.random_range(0..=3)) * rng.random_range(1..=9),
            )]);
        }

        let mut attacker = party(&mut rng, &types, 4);

        // One seed in forty is a boundary probe rather than a roll, and it is
        // aimed at the extreme point of the stat table: a pack of Deathstars at
        // high Weapons, whose single shot of up to 5,300,000 outweighs an
        // unresearched Espionage Probe's hundred hitpoints by fifty thousand.
        // That corner is where the damage budget and the shot budget come
        // apart — few enough hulls to be over any margin measured in damage,
        // too few to have aimed at a swarm — and a roll out of a twenty-odd-row
        // table with independent fleet sizes wanders into it perhaps once in
        // ten thousand seeds, which is a test that catches the bug on somebody
        // else's branch.
        //
        // The pack's size is bisected rather than rolled, because the smallest
        // pack the rule will still speak for is by definition the hardest
        // battle it claims, and standing on that line finds an unsound
        // condition whichever of the conditions is the one holding it up.
        let swarm: u32 = defender.entities.values().sum();
        if seed % 40 == 1 && (1..=PROBE_SWARM_LIMIT).contains(&swarm) {
            // The ratio being aimed at is one side's weapon power against the
            // other side's hitpoints, so the probe takes it to both extremes the
            // engine can express: the swarm is unresearched and the pack is at
            // the top half of the Weapons range. A researched swarm is the same
            // shape with the corner rounded off — one level of Armour on a probe
            // is ten more hitpoints, and ten more hitpoints is a bigger pack,
            // and a bigger pack is more shots.
            defender.technology = Technology::default();
            let mut pack = attacker.clone();
            pack.technology.weapon = rng.random_range(128..=255);

            if let Some(count) =
                smallest_pack_the_rule_speaks_for(&combat, &pack, &defender, use_rapid_fire, seed)
            {
                pack.entities = HashMap::from([(DEATHSTAR, count)]);
                attacker = pack;
            }
        }

        let units: u32 =
            attacker.entities.values().sum::<u32>() + defender.entities.values().sum::<u32>();
        if units > UNIT_BUDGET {
            continue;
        }

        let mut instant_rng = SmallRng::seed_from_u64(seed);
        let instant = combat.simulate_single(
            &attacker,
            &defender,
            use_rapid_fire,
            false,
            &mut instant_rng,
        );
        if instant.rounds != 0 {
            continue;
        }
        // A side that brought nothing is over before it starts, and reports 0
        // rounds without the rule having said anything.
        if attacker.entities.values().sum::<u32>() == 0
            || defender.entities.values().sum::<u32>() == 0
        {
            continue;
        }
        triggered += 1;

        let mut rounds_rng = SmallRng::seed_from_u64(seed);
        let fought = combat.simulate_single_through_the_rounds(
            &attacker,
            &defender,
            use_rapid_fire,
            false,
            &mut rounds_rng,
        );

        assert!(
            instant.outcome == fought.outcome,
            "seed {seed} (rapid fire {use_rapid_fire}): the two paths disagree about who won\n{attacker:?}\n{defender:?}"
        );
        assert_eq!(
            instant.attacker_losses, fought.attacker_losses,
            "seed {seed} (rapid fire {use_rapid_fire}): attacker losses differ\n{attacker:?}\n{defender:?}"
        );
        assert_eq!(
            instant.defender_losses, fought.defender_losses,
            "seed {seed} (rapid fire {use_rapid_fire}): defender losses differ\n{attacker:?}\n{defender:?}"
        );
    }

    assert!(
        triggered >= TRIGGERS_EXPECTED,
        "the rule fired on {triggered} of these battles, so this test proved almost nothing"
    );
}
