//! Lifeform research, asserted through `Simulator::simulate_multiple` rather
//! than against the stat arithmetic — the arithmetic is pinned in
//! `ModifiedStats::calculate` and in `combat-types`, and what matters here is
//! that a battle actually resolves differently, on the right ships and the
//! right side.
//!
//! Every test uses the shared fixture, the same one `class_bonuses.rs` asks its
//! own question of, and for the same reason: it is decided by a single number
//! and is otherwise free of randomness. See [`common`], which describes it and
//! says why it answers the question.

mod common;

use combat_core::Simulator;
use combat_types::{
    BuiltinLifeformTechs, CombatRequest, EntityType, LifeformBonus, LifeformBonuses,
    LifeformTechTable, PartyData, PartySlot, Technology,
};
use common::{
    FIGHTERS, LARGE_SHIELD_DOME, LIGHT_FIGHTER, Outcome, SIMULATIONS, WEAPONS_THAT_BOUNCE,
    WEAPONS_THAT_LAND, summarise,
};
use std::collections::HashMap;

const CRUISER: EntityType = 206;

/// What one researched level of Weapons is worth as a percentage, and so the
/// lifeform bonus that has to be worth exactly as much.
const ONE_LEVEL: f32 = 10.0;

/// The fixture under lifeform research: one side's bonuses are what these tests
/// vary, and the technology levels stay where the fixture put them.
fn resolve(
    weapons: u8,
    attacker_lifeform: LifeformBonuses,
    defender_lifeform: LifeformBonuses,
) -> Outcome {
    let request = CombatRequest {
        attacker: PartyData {
            technology: Technology {
                weapon: weapons,
                ..Default::default()
            },
            entities: HashMap::from([(LIGHT_FIGHTER, FIGHTERS)]),
            lifeform: attacker_lifeform,
        },
        defender: PartyData {
            technology: Technology::default(),
            entities: HashMap::from([(LARGE_SHIELD_DOME, 1)]),
            lifeform: defender_lifeform,
        },
        simulations: SIMULATIONS,
        ..Default::default()
    };

    summarise(&Simulator::new().simulate_multiple(&request))
}

/// A bonus on one ship type and nothing else.
fn on(entity_type: EntityType, percent: f32) -> LifeformBonuses {
    LifeformBonuses::from_iter([(entity_type, LifeformBonus::uniform(percent))])
}

/// The acceptance criterion, at battle level: the bonus adds to the base stat
/// in the same bracket as technology, so ten percent of lifeform is worth
/// exactly the researched level it equals — not more, which compounding would
/// give, and not nothing.
#[test]
fn a_lifeform_bonus_is_worth_the_researched_level_it_equals() {
    let with_lifeform = resolve(
        WEAPONS_THAT_BOUNCE,
        on(LIGHT_FIGHTER, ONE_LEVEL),
        LifeformBonuses::default(),
    );
    let one_more_level = resolve(
        WEAPONS_THAT_LAND,
        LifeformBonuses::default(),
        LifeformBonuses::default(),
    );
    let without = resolve(
        WEAPONS_THAT_BOUNCE,
        LifeformBonuses::default(),
        LifeformBonuses::default(),
    );

    assert_eq!(with_lifeform, one_more_level, "worth exactly one level");
    assert_ne!(with_lifeform, without, "and that level is worth having");

    // Which way round it went, so a reader does not have to derive it: the
    // buffed fighters kill the dome in the first round, the others never
    // scratch it.
    assert_eq!(with_lifeform.attacker_wins, SIMULATIONS);
    assert_eq!(with_lifeform.rounds, 1);
    assert_eq!(without.draws, SIMULATIONS);
}

/// The whole difference between a lifeform bonus and a class: it names one ship
/// type. A fleet of Light Fighters gains nothing from research that buffs
/// Cruisers, however large the number.
#[test]
fn a_bonus_on_another_ship_type_leaves_this_fleet_alone() {
    let cruiser_research = resolve(
        WEAPONS_THAT_BOUNCE,
        on(CRUISER, 500.0),
        LifeformBonuses::default(),
    );

    assert_eq!(
        cruiser_research,
        resolve(
            WEAPONS_THAT_BOUNCE,
            LifeformBonuses::default(),
            LifeformBonuses::default()
        )
    );
    assert_eq!(cruiser_research.draws, SIMULATIONS);
}

/// A mixed fleet, which is where a single flat percentage would have gone
/// wrong: the buffed half of the fleet has to move and the other half stay
/// exactly where it was.
#[test]
fn a_mixed_fleet_moves_only_the_ship_the_research_names() {
    // A Cruiser's 760 clears the dome's 1% line on its own, but one Cruiser
    // only takes 7% off a shield that regenerates every round, so it can never
    // bring the dome down alone. The battle is decided by whether the fighters
    // join in.
    let mixed = |lifeform: LifeformBonuses| {
        let request = CombatRequest {
            attacker: PartyData {
                technology: Technology {
                    weapon: WEAPONS_THAT_BOUNCE,
                    ..Default::default()
                },
                entities: HashMap::from([(LIGHT_FIGHTER, FIGHTERS), (CRUISER, 1)]),
                lifeform,
            },
            defender: PartyData {
                technology: Technology::default(),
                entities: HashMap::from([(LARGE_SHIELD_DOME, 1)]),
                ..Default::default()
            },
            simulations: SIMULATIONS,
            ..Default::default()
        };

        summarise(&Simulator::new().simulate_multiple(&request))
    };

    let none = mixed(LifeformBonuses::default());
    let on_the_cruiser = mixed(on(CRUISER, ONE_LEVEL));
    let on_the_fighter = mixed(on(LIGHT_FIGHTER, ONE_LEVEL));

    assert_eq!(
        on_the_cruiser, none,
        "buffing the ship that already lands its shots decides nothing",
    );
    assert_ne!(
        on_the_fighter, none,
        "buffing the ship that was bouncing decides the battle",
    );
    assert_eq!(on_the_fighter.rounds, 1);
}

/// Bonuses are per side. The attacker's research adds to shields too — to *his*
/// fighters. Had it reached across, the dome would be shielded at 11,000, the
/// bounce line would move to 110, and the shot that wins this battle would
/// bounce off it.
#[test]
fn an_attackers_research_never_reaches_the_defender() {
    let attacker_buffed = resolve(
        WEAPONS_THAT_LAND,
        on(LARGE_SHIELD_DOME, ONE_LEVEL),
        LifeformBonuses::default(),
    );

    assert_eq!(attacker_buffed.attacker_wins, SIMULATIONS);
    assert_eq!(
        attacker_buffed.defender_losses,
        HashMap::from([(LARGE_SHIELD_DOME, 1)])
    );
}

/// The mirror image, and the same rule: the defender's own research does raise
/// the defender's shields. Obsidian Shield Reinforcement is the one lifeform
/// research in the game that touches defence, and this is what it buys.
#[test]
fn a_defenders_research_raises_the_defenders_shields() {
    let defended = resolve(
        WEAPONS_THAT_LAND,
        LifeformBonuses::default(),
        on(LARGE_SHIELD_DOME, ONE_LEVEL),
    );

    assert_ne!(
        defended,
        resolve(
            WEAPONS_THAT_LAND,
            LifeformBonuses::default(),
            LifeformBonuses::default()
        ),
        "ten percent of shield puts the bounce line above the incoming shot",
    );
    assert_eq!(defended.draws, SIMULATIONS);
}

/// The case every request written before lifeforms existed takes: no lifeform
/// data must resolve exactly as it did then. Held at the level where a single
/// level either way flips the battle, so an accidental bonus of any size would
/// show.
#[test]
fn a_request_with_no_lifeform_data_resolves_as_it_always_did() {
    assert_eq!(
        resolve(
            WEAPONS_THAT_BOUNCE,
            LifeformBonuses::default(),
            LifeformBonuses::default()
        )
        .draws,
        SIMULATIONS,
    );
    assert_eq!(
        resolve(
            WEAPONS_THAT_LAND,
            LifeformBonuses::default(),
            LifeformBonuses::default()
        )
        .attacker_wins,
        SIMULATIONS,
    );
}

/// The slot path is a second route through `simulate_multiple`. Lifeform
/// research is per player and the slots of an ACS attack are different players,
/// so each slot fights under its own — which is exactly what carrying the
/// bonuses on `PartyData` buys, and it is worth asserting rather than assuming.
///
/// Counted in shield damage rather than in who won, because the count is what
/// distinguishes *one* slot's research reaching the battle from it leaking into
/// the other slot as well. [`SMALL_SLOT`] fighters per slot is small enough
/// that the dome's shield never falls: every shot either bounces for nothing or
/// takes exactly 1% of a 10,000 shield, so a round's shield damage is a direct
/// count of how many fighters were armed, and no shot ever reaches the hull.
#[test]
fn each_slot_fights_under_its_own_research() {
    /// Few enough that one slot cannot strip the dome's shield in a round, and
    /// so few that two slots cannot either.
    const SMALL_SLOT: u32 = 30;

    /// What one armed fighter takes off a 10,000 shield: a shot worth 100 is
    /// exactly 1% of it.
    const PER_ARMED_FIGHTER: u64 = 100;

    let slot = |id: &str, lifeform: LifeformBonuses| PartySlot {
        id: id.to_string(),
        name: None,
        data: PartyData {
            technology: Technology {
                weapon: WEAPONS_THAT_BOUNCE,
                ..Default::default()
            },
            entities: HashMap::from([(LIGHT_FIGHTER, SMALL_SLOT)]),
            lifeform,
        },
    };

    let shield_damage_in_the_first_round = |a1: LifeformBonuses, a2: LifeformBonuses| {
        let request = CombatRequest {
            // The flat fields are ignored once slots are present, but the
            // engine still reads them for downscaling, so they mirror the slots.
            attacker: PartyData {
                technology: Technology {
                    weapon: WEAPONS_THAT_BOUNCE,
                    ..Default::default()
                },
                entities: HashMap::from([(LIGHT_FIGHTER, SMALL_SLOT * 2)]),
                ..Default::default()
            },
            defender: PartyData {
                technology: Technology::default(),
                entities: HashMap::from([(LARGE_SHIELD_DOME, 1)]),
                ..Default::default()
            },
            attacker_slots: Some(vec![slot("A1", a1), slot("A2", a2)]),
            defender_slots: Some(vec![PartySlot {
                id: "D1".to_string(),
                name: None,
                data: PartyData {
                    technology: Technology::default(),
                    entities: HashMap::from([(LARGE_SHIELD_DOME, 1)]),
                    ..Default::default()
                },
            }]),
            simulations: 1,
            ..Default::default()
        };

        let results = Simulator::new().simulate_multiple(&request);
        results.results[0]
            .round_details
            .as_ref()
            .expect("the engine records every round")[0]
            .attacker_shield_damage
            .expect("shield damage is recorded")
    };

    assert_eq!(
        shield_damage_in_the_first_round(LifeformBonuses::default(), LifeformBonuses::default()),
        0,
        "unarmed, every shot bounces",
    );
    assert_eq!(
        shield_damage_in_the_first_round(on(LIGHT_FIGHTER, ONE_LEVEL), LifeformBonuses::default()),
        u64::from(SMALL_SLOT) * PER_ARMED_FIGHTER,
        "one slot's research arms that slot — and only that slot",
    );
    assert_eq!(
        shield_damage_in_the_first_round(
            on(LIGHT_FIGHTER, ONE_LEVEL),
            on(LIGHT_FIGHTER, ONE_LEVEL)
        ),
        u64::from(SMALL_SLOT * 2) * PER_ARMED_FIGHTER,
        "and the second slot's research is its own",
    );
}

/// The built-in table reaching a battle, which is the path a caller who has not
/// computed its own percentages takes: researched levels in, a decided battle
/// out. 34 levels of the Human Light Fighter research is 10.2%, just over the
/// one level the shot needs.
#[test]
fn researched_levels_from_the_builtin_table_decide_a_battle() {
    const HUMAN_LIGHT_FIGHTER: u16 = 11209;

    let bonuses = BuiltinLifeformTechs.resolve(&HashMap::from([(HUMAN_LIGHT_FIGHTER, 34)]), 0.0);

    assert_eq!(
        resolve(WEAPONS_THAT_BOUNCE, bonuses, LifeformBonuses::default()).attacker_wins,
        SIMULATIONS,
    );
}
