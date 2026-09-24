// This boundary uses the shared battle constants, not the aggregate simulator helpers.
#[allow(dead_code)]
mod common;
use combat_core::RoundOutcome as CombatOutcome;
use combat_core::{Combat, ReportedBattle, ReportedSlot, ReportedUnit};
use common::{FIGHTERS, LARGE_SHIELD_DOME, LIGHT_FIGHTER};
use std::collections::BTreeMap;

fn battle(weapon: u32) -> ReportedBattle {
    ReportedBattle {
        attackers: vec![ReportedSlot {
            units: BTreeMap::from([(
                LIGHT_FIGHTER,
                ReportedUnit {
                    count: FIGHTERS,
                    weapon,
                    shield: 100.0,
                    hull: 400.0,
                },
            )]),
        }],
        defenders: vec![ReportedSlot {
            units: BTreeMap::from([(
                LARGE_SHIELD_DOME,
                ReportedUnit {
                    count: 1,
                    weapon: 1,
                    shield: 10000.0,
                    hull: 10000.0,
                },
            )]),
        }],
        rapid_fire: false,
    }
}

#[test]
fn reported_stats_enter_the_existing_bounce_rule_without_modifiers() {
    let combat = Combat::new();
    let bounce = combat.simulate_reported(&battle(99)).unwrap();
    assert_eq!(bounce.outcome, CombatOutcome::Draw);
    assert_eq!(bounce.rounds, 6);
    let hit = combat.simulate_reported(&battle(100)).unwrap();
    assert_eq!(hit.outcome, CombatOutcome::AttackersWin);
    assert_eq!(hit.rounds, 1);
    assert_eq!(hit.defender_losses[&LARGE_SHIELD_DOME], 1);
}

#[test]
fn invalid_reported_inputs_are_rejected_before_allocating_fleets() {
    let combat = Combat::new();
    let mut input = battle(100);
    input.attackers[0]
        .units
        .get_mut(&LIGHT_FIGHTER)
        .unwrap()
        .hull = f32::NAN;
    assert!(combat.simulate_reported(&input).is_err());
    input = battle(100);
    input.attackers[0]
        .units
        .get_mut(&LIGHT_FIGHTER)
        .unwrap()
        .count = u32::MAX;
    assert!(combat.simulate_reported(&input).is_err());
    input = battle(100);
    input.attackers[0].units.insert(
        65535,
        ReportedUnit {
            count: 1,
            weapon: 1,
            shield: 1.0,
            hull: 1.0,
        },
    );
    assert!(combat.simulate_reported(&input).is_err());
}

#[test]
fn different_slots_keep_their_own_reported_stats() {
    let mut input = battle(100);
    input.attackers.push(ReportedSlot {
        units: BTreeMap::from([(
            LIGHT_FIGHTER,
            ReportedUnit {
                count: 1,
                weapon: 0,
                shield: 100.0,
                hull: 400.0,
            },
        )]),
    });
    let result = Combat::new().simulate_reported(&input).unwrap();
    assert_eq!(result.outcome, CombatOutcome::AttackersWin);
    assert_eq!(result.rounds, 1);
    let slots = result.attacker_slots.unwrap();
    assert_eq!(
        slots
            .iter()
            .find(|s| s.slot_id == "A1")
            .unwrap()
            .remaining
            .get(&LIGHT_FIGHTER),
        Some(&FIGHTERS)
    );
    assert_eq!(
        slots
            .iter()
            .find(|s| s.slot_id == "A2")
            .unwrap()
            .remaining
            .get(&LIGHT_FIGHTER),
        Some(&1)
    );
}
