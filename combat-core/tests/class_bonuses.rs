//! Player and alliance classes, asserted through `Simulator::simulate_multiple`
//! rather than against the level arithmetic — the arithmetic is pinned in
//! `combat-types`, and what matters here is that a battle actually resolves
//! differently.
//!
//! Every test uses one fixture, chosen because it is decided by a single
//! number and is otherwise free of randomness. See [`resolve`].

use combat_core::{ReportBuilder, Simulator};
use combat_types::{
    AllianceClass, CombatRequest, CombatResults, EntityType, FleetComposition, PartyData,
    PlayerBonuses, PlayerClass, Technology,
};
use std::collections::HashMap;

const LIGHT_FIGHTER: EntityType = 204;
const LARGE_SHIELD_DOME: EntityType = 408;

/// Enough fighters to bring a dome down inside one round once their shots
/// register at all — the shield absorbs the first hundred of them.
const FIGHTERS: u32 = 250;

/// The highest Weapons level whose shots still bounce: a Light Fighter's 50
/// becomes 95, and 95 is under 1% of the dome's 10,000 shield.
const WEAPONS_THAT_BOUNCE: u8 = 9;

/// One level up, the shot is worth exactly 100 and registers.
const WEAPONS_THAT_LAND: u8 = 10;

/// The fixture repeats, and a handful of runs of a battle with no randomness in
/// it is enough — if anything here were random these tests would flap loudly
/// rather than quietly.
const SIMULATIONS: u32 = 5;

/// Everything about this battle a class bonus could possibly move.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    attacker_wins: u32,
    defender_wins: u32,
    draws: u32,
    rounds: u8,
    attacker_losses: FleetComposition,
    defender_losses: FleetComposition,
}

/// Resolve the fixture: [`FIGHTERS`] Light Fighters attacking a single Large
/// Shield Dome.
///
/// The dome's weapon is 1, which is under 1% of a Light Fighter's shield, so
/// its shots bounce and the attacker cannot lose a ship however long the battle
/// lasts. Nothing else in the battle rolls a die that matters: there is one
/// target, so target selection is fixed, neither ship has rapid fire against
/// the other, and the only unit that ever takes hull damage dies within the
/// round it starts taking them. Every simulation therefore agrees, and the
/// result turns on one comparison — whether a fighter's shot clears 1% of the
/// dome's shield:
///
/// - below it, the shot bounces off entirely, the dome is untouchable, and six
///   rounds end in a draw with nobody having lost anything;
/// - at or above it, the shield comes down inside the first round and 250
///   fighters flatten the dome before it can do anything about it.
///
/// The dome's shield is 10,000 at Shielding 0, putting the line at 100 — which
/// is a Light Fighter at Weapons 10, and one effective level either side of it
/// changes the answer.
fn resolve(
    weapons: u8,
    attacker_bonuses: Option<PlayerBonuses>,
    shielding: u8,
    defender_bonuses: Option<PlayerBonuses>,
) -> Outcome {
    let request = CombatRequest {
        attacker: PartyData {
            technology: Technology {
                weapon: weapons,
                ..Default::default()
            },
            entities: HashMap::from([(LIGHT_FIGHTER, FIGHTERS)]),
        },
        defender: PartyData {
            technology: Technology {
                shield: shielding,
                ..Default::default()
            },
            entities: HashMap::from([(LARGE_SHIELD_DOME, 1)]),
        },
        attacker_bonuses,
        defender_bonuses,
        simulations: SIMULATIONS,
        ..Default::default()
    };

    summarise(&Simulator::new().simulate_multiple(&request))
}

fn summarise(results: &CombatResults) -> Outcome {
    let first = &results.results[0];
    Outcome {
        attacker_wins: results.attacker_wins,
        defender_wins: results.defender_wins,
        draws: results.draws,
        rounds: first.rounds,
        attacker_losses: first.attacker_losses.clone(),
        defender_losses: first.defender_losses.clone(),
    }
}

/// A player carrying nothing but the two classes named.
fn classes(player_class: PlayerClass, alliance_class: AllianceClass) -> PlayerBonuses {
    PlayerBonuses {
        player_class,
        alliance_class,
        ..Default::default()
    }
}

/// The acceptance criterion, stated exactly: a General resolves as the same
/// player with two more levels researched, not as anything approximating it.
#[test]
fn a_general_attacker_resolves_as_two_more_researched_levels() {
    let general = resolve(
        WEAPONS_THAT_LAND - 2,
        Some(classes(PlayerClass::General, AllianceClass::None)),
        0,
        None,
    );
    let two_more_levels = resolve(WEAPONS_THAT_LAND, None, 0, None);
    let classless = resolve(WEAPONS_THAT_LAND - 2, None, 0, None);

    assert_eq!(general, two_more_levels, "a General is two levels");
    assert_ne!(general, classless, "and the two levels are worth having");

    // Which way round it went, so a reader does not have to derive it: the
    // General's fighters kill the dome in the first round, the classless ones
    // never scratch it.
    assert_eq!(general.attacker_wins, SIMULATIONS);
    assert_eq!(general.rounds, 1);
    assert_eq!(classless.draws, SIMULATIONS);
}

#[test]
fn a_warrior_alliance_resolves_as_one_more_researched_level() {
    let warrior = resolve(
        WEAPONS_THAT_LAND - 1,
        Some(classes(PlayerClass::None, AllianceClass::Warrior)),
        0,
        None,
    );

    assert_eq!(warrior, resolve(WEAPONS_THAT_LAND, None, 0, None));
    assert_ne!(warrior, resolve(WEAPONS_THAT_LAND - 1, None, 0, None));
}

/// Both classes feed the same pipeline in the live game, so a General in a
/// Warrior alliance is three levels, not two, and not two applied twice.
#[test]
fn a_general_in_a_warrior_alliance_resolves_as_three_more_levels() {
    let both = resolve(
        WEAPONS_THAT_LAND - 3,
        Some(classes(PlayerClass::General, AllianceClass::Warrior)),
        0,
        None,
    );

    assert_eq!(both, resolve(WEAPONS_THAT_LAND, None, 0, None));
    assert_ne!(both, resolve(WEAPONS_THAT_LAND - 3, None, 0, None));
}

/// Held at the level where a single extra level would flip the battle, so a
/// class that is not supposed to touch combat cannot slip one in unnoticed.
#[test]
fn the_classes_without_a_combat_effect_change_nothing() {
    let classless = resolve(WEAPONS_THAT_BOUNCE, None, 0, None);

    for player_class in [
        PlayerClass::None,
        PlayerClass::Collector,
        PlayerClass::Discoverer,
    ] {
        for alliance_class in [
            AllianceClass::None,
            AllianceClass::Trader,
            AllianceClass::Researcher,
        ] {
            assert_eq!(
                resolve(
                    WEAPONS_THAT_BOUNCE,
                    Some(classes(player_class, alliance_class)),
                    0,
                    None,
                ),
                classless,
                "{player_class:?} + {alliance_class:?} changed the battle",
            );
        }
    }
}

/// Bonuses are per side. The attacker's General adds two levels of Shielding
/// too — to *his* fighters. Had it reached across, the dome would be shielded
/// at 12,000, the bounce line would move to 120, and the same shot that wins
/// this battle would bounce off.
#[test]
fn an_attackers_class_never_reaches_the_defender() {
    let general = resolve(
        WEAPONS_THAT_LAND - 2,
        Some(classes(PlayerClass::General, AllianceClass::None)),
        0,
        None,
    );

    assert_eq!(general.attacker_wins, SIMULATIONS);
    assert_eq!(general.defender_losses, HashMap::from([(408, 1)]));
}

/// The mirror image, and the same rule: the defender's own class does raise
/// the defender's shields, by exactly two levels of Shielding.
#[test]
fn a_defenders_class_raises_the_defenders_shields() {
    let general_defender = resolve(
        WEAPONS_THAT_LAND,
        None,
        0,
        Some(classes(PlayerClass::General, AllianceClass::None)),
    );

    assert_eq!(
        general_defender,
        resolve(WEAPONS_THAT_LAND, None, 2, None),
        "a General defender is two levels of Shielding"
    );
    assert_ne!(
        general_defender,
        resolve(WEAPONS_THAT_LAND, None, 0, None),
        "and those two levels save the dome"
    );
    assert_eq!(general_defender.draws, SIMULATIONS);
}

/// A bonus block naming no class must be worth exactly as much as no block at
/// all — the case every request that has never heard of classes takes.
#[test]
fn bonuses_naming_no_class_resolve_as_no_bonuses() {
    let empty = Some(PlayerBonuses::default());

    assert_eq!(
        resolve(WEAPONS_THAT_LAND, empty.clone(), 0, empty),
        resolve(WEAPONS_THAT_LAND, None, 0, None)
    );
}

/// The report has to describe the battle that happened. Reporting the
/// researched levels next to a result produced at three levels higher is the
/// kind of disagreement that makes a simulator look broken when it is not.
#[test]
fn the_report_names_the_levels_the_battle_was_fought_at() {
    let request = CombatRequest {
        attacker: PartyData {
            technology: Technology {
                weapon: 10,
                shield: 10,
                armour: 10,
                ..Default::default()
            },
            entities: HashMap::from([(LIGHT_FIGHTER, FIGHTERS)]),
        },
        defender: PartyData {
            technology: Technology::default(),
            entities: HashMap::from([(LARGE_SHIELD_DOME, 1)]),
        },
        attacker_bonuses: Some(classes(PlayerClass::General, AllianceClass::Warrior)),
        simulations: 1,
        ..Default::default()
    };

    let results = Simulator::new().simulate_multiple(&request);
    let report = ReportBuilder::new().build_summary_report(&request, &results);

    assert_eq!(report.attacker.technology.weapon, 13);
    assert_eq!(report.attacker.technology.shield, 13);
    assert_eq!(report.attacker.technology.armour, 13);
    assert_eq!(
        report.defender.technology,
        Technology::default(),
        "the defender has no class and gains nothing"
    );
}
