/// Combat scaling utilities for large battles
///
/// For battles with 10M+ ships, we can downscale the fleet sizes
/// and scale results back up for massive performance gains with
/// minimal accuracy loss.
use combat_types::{
    FleetComposition, PartyData, RoundComposition, RoundDetails, SimulationResult, SlotResult,
};
use std::collections::HashMap;

/// Threshold above which we apply downscaling
pub const DOWNSCALE_THRESHOLD: usize = 10_000_000;

/// Calculate total ship count in a fleet
pub fn total_ships(fleet: &FleetComposition) -> usize {
    fleet.values().map(|&count| count as usize).sum()
}

pub fn upscale_round_details(
    details: &Option<Vec<RoundDetails>>,
    factor: usize,
) -> Option<Vec<RoundDetails>> {
    match details {
        None => None,
        Some(v) => {
            if factor <= 1 {
                return Some(v.clone());
            }
            let mut out = Vec::with_capacity(v.len());
            for d in v.iter() {
                out.push(RoundDetails {
                    round_number: d.round_number,
                    attackers_start: d.attackers_start.saturating_mul(factor as u32),
                    defenders_start: d.defenders_start.saturating_mul(factor as u32),
                    attackers_destroyed: d.attackers_destroyed.saturating_mul(factor as u32),
                    defenders_destroyed: d.defenders_destroyed.saturating_mul(factor as u32),
                    attackers_end: d.attackers_end.saturating_mul(factor as u32),
                    defenders_end: d.defenders_end.saturating_mul(factor as u32),
                    attacker_damage: d.attacker_damage.map(|x| x.saturating_mul(factor as u64)),
                    defender_damage: d.defender_damage.map(|x| x.saturating_mul(factor as u64)),
                    attacker_shots: d.attacker_shots.map(|x| x.saturating_mul(factor as u64)),
                    defender_shots: d.defender_shots.map(|x| x.saturating_mul(factor as u64)),
                    attacker_shield_damage: d
                        .attacker_shield_damage
                        .map(|x| x.saturating_mul(factor as u64)),
                    defender_shield_damage: d
                        .defender_shield_damage
                        .map(|x| x.saturating_mul(factor as u64)),
                });
            }
            Some(out)
        }
    }
}

fn upscale_round_comp(rc: &RoundComposition, factor: usize) -> RoundComposition {
    if factor <= 1 {
        return rc.clone();
    }
    let mul = |m: &FleetComposition| -> FleetComposition {
        m.iter().map(|(&t, &c)| (t, c * factor as u32)).collect()
    };
    RoundComposition {
        round_number: rc.round_number,
        attacker_by_type_start: mul(&rc.attacker_by_type_start),
        defender_by_type_start: mul(&rc.defender_by_type_start),
        attacker_by_type_destroyed: mul(&rc.attacker_by_type_destroyed),
        defender_by_type_destroyed: mul(&rc.defender_by_type_destroyed),
        attacker_by_type_end: mul(&rc.attacker_by_type_end),
        defender_by_type_end: mul(&rc.defender_by_type_end),
    }
}

pub fn upscale_round_compositions(
    comps: &Option<Vec<RoundComposition>>,
    factor: usize,
) -> Option<Vec<RoundComposition>> {
    match comps {
        None => None,
        Some(v) => {
            if factor <= 1 {
                return Some(v.clone());
            }
            Some(v.iter().map(|rc| upscale_round_comp(rc, factor)).collect())
        }
    }
}

pub fn upscale_round_compositions_by_slot(
    by_slot: &Option<std::collections::HashMap<String, Vec<RoundComposition>>>,
    factor: usize,
) -> Option<std::collections::HashMap<String, Vec<RoundComposition>>> {
    match by_slot {
        None => None,
        Some(map) => {
            if factor <= 1 {
                return Some(map.clone());
            }
            let mut out = std::collections::HashMap::with_capacity(map.len());
            for (k, v) in map.iter() {
                out.insert(
                    k.clone(),
                    v.iter().map(|rc| upscale_round_comp(rc, factor)).collect(),
                );
            }
            Some(out)
        }
    }
}

pub fn upscale_slot_results(
    slots: &[SlotResult],
    factor: usize,
    original_per_slot: &HashMap<String, FleetComposition>,
    prefix: char,
) -> Vec<SlotResult> {
    if factor <= 1 {
        return slots.to_vec();
    }
    slots
        .iter()
        .map(|s| {
            // Normalize key: simulate_single_with_slots labels as "A1"/"D1".
            // We accept either exact match or without first char; prefer exact.
            let empty: FleetComposition = HashMap::new();
            let orig = match original_per_slot.get(&s.slot_id) {
                Some(o) => o,
                None => {
                    let trimmed = s.slot_id.trim_start_matches(prefix).to_string();
                    original_per_slot.get(&trimmed).unwrap_or(&empty)
                }
            };

            let mut remaining: FleetComposition = HashMap::new();
            let mut initial: FleetComposition = HashMap::new();

            for (&t, &orig_c) in orig.iter() {
                let scaled_losses = s.losses.get(&t).copied().unwrap_or(0) * factor as u32;
                let rem = if scaled_losses == 0 {
                    orig_c
                } else {
                    orig_c.saturating_sub(scaled_losses)
                };
                if rem > 0 {
                    remaining.insert(t, rem);
                }
                if orig_c > 0 {
                    initial.insert(t, orig_c);
                }
            }

            // Ensure losses = initial - remaining
            let mut losses: FleetComposition = HashMap::new();
            for (&t, &init_c) in initial.iter() {
                let rem = remaining.get(&t).copied().unwrap_or(0);
                if init_c > rem {
                    losses.insert(t, init_c - rem);
                }
            }

            SlotResult {
                slot_id: s.slot_id.clone(),
                initial,
                losses,
                remaining,
            }
        })
        .collect()
}

/// Determine if a battle should be downscaled
pub fn should_downscale(attacker: &PartyData, defender: &PartyData) -> bool {
    let total = total_ships(&attacker.entities) + total_ships(&defender.entities);
    total > DOWNSCALE_THRESHOLD
}

/// Calculate appropriate downscale factor based on fleet size
pub fn calculate_downscale_factor(attacker: &PartyData, defender: &PartyData) -> usize {
    let total = total_ships(&attacker.entities) + total_ships(&defender.entities);

    if total <= DOWNSCALE_THRESHOLD {
        return 1; // No downscaling
    }

    // Progressive downscaling
    if total > 100_000_000 {
        return 100; // 100M+ ships: downscale by 100x
    }
    if total >= 50_000_000 {
        return 50; // 50M+ ships: downscale by 50x
    }
    if total >= 10_000_000 {
        return 10; // 10M+ ships: downscale by 10x
    }

    1 // Default: no downscaling
}

/// Downscale a fleet by the given factor
pub fn downscale_fleet(fleet: &FleetComposition, factor: usize) -> FleetComposition {
    if factor <= 1 {
        return fleet.clone();
    }

    fleet
        .iter()
        .map(|(&entity_type, &count)| {
            // Always keep at least 1 ship of each type
            let scaled_count = (count / factor as u32).max(1);
            (entity_type, scaled_count)
        })
        .collect()
}

/// Downscale party data
pub fn downscale_party(party: &PartyData, factor: usize) -> PartyData {
    PartyData {
        technology: party.technology,
        entities: downscale_fleet(&party.entities, factor),
    }
}

/// Scale up simulation results with precision preservation
/// This version tries to preserve the original fleet counts for survivors
pub fn upscale_result_with_originals(
    result: &SimulationResult,
    factor: usize,
    original_attacker: &FleetComposition,
    original_defender: &FleetComposition,
) -> SimulationResult {
    if factor <= 1 {
        return result.clone();
    }

    // Scale up losses normally
    let attacker_losses = upscale_fleet(&result.attacker_losses, factor);
    let defender_losses = upscale_fleet(&result.defender_losses, factor);

    // For remaining ships, preserve original counts where possible
    // If a ship type didn't lose any in the scaled sim, restore original count
    let attacker_remaining = restore_precision_fleet(
        &result.attacker_remaining,
        &result.attacker_losses,
        original_attacker,
        factor,
    );

    let defender_remaining = restore_precision_fleet(
        &result.defender_remaining,
        &result.defender_losses,
        original_defender,
        factor,
    );

    // Scale up debris field proportionally
    let debris_field = combat_types::DebrisField {
        metal: result.debris_field.metal * factor as u64,
        crystal: result.debris_field.crystal * factor as u64,
    };

    // Scale up loot proportionally
    let loot = combat_types::PlanetResources {
        metal: result.loot.metal * factor as u64,
        crystal: result.loot.crystal * factor as u64,
        deuterium: result.loot.deuterium * factor as u64,
    };

    SimulationResult {
        outcome: result.outcome.clone(),
        rounds: result.rounds,
        attacker_losses,
        defender_losses,
        attacker_remaining,
        defender_remaining,
        debris_field,
        loot,
        attacker_profit: result.attacker_profit * factor as i64,
        defender_profit: result.defender_profit * factor as i64,
        round_details: result.round_details.clone(),
        round_compositions: result.round_compositions.clone(),
        round_compositions_by_slot: result.round_compositions_by_slot.clone(),
        attacker_slots: result.attacker_slots.clone(),
        defender_slots: result.defender_slots.clone(),
    }
}

/// Scale up simulation results (legacy, for compatibility)
pub fn upscale_result(result: &SimulationResult, factor: usize) -> SimulationResult {
    if factor <= 1 {
        return result.clone();
    }

    // Scale up debris field proportionally
    let debris_field = combat_types::DebrisField {
        metal: result.debris_field.metal * factor as u64,
        crystal: result.debris_field.crystal * factor as u64,
    };

    // Scale up loot proportionally
    let loot = combat_types::PlanetResources {
        metal: result.loot.metal * factor as u64,
        crystal: result.loot.crystal * factor as u64,
        deuterium: result.loot.deuterium * factor as u64,
    };

    SimulationResult {
        outcome: result.outcome.clone(),
        rounds: result.rounds,
        attacker_losses: upscale_fleet(&result.attacker_losses, factor),
        defender_losses: upscale_fleet(&result.defender_losses, factor),
        attacker_remaining: upscale_fleet(&result.attacker_remaining, factor),
        defender_remaining: upscale_fleet(&result.defender_remaining, factor),
        debris_field,
        loot,
        attacker_profit: result.attacker_profit * factor as i64,
        defender_profit: result.defender_profit * factor as i64,
        round_details: result.round_details.clone(),
        round_compositions: result.round_compositions.clone(),
        round_compositions_by_slot: result.round_compositions_by_slot.clone(),
        attacker_slots: result.attacker_slots.clone(),
        defender_slots: result.defender_slots.clone(),
    }
}

/// Restore precision by using original counts where appropriate
fn restore_precision_fleet(
    _remaining: &FleetComposition,
    losses: &FleetComposition,
    original: &FleetComposition,
    _factor: usize,
) -> FleetComposition {
    original
        .iter()
        .filter_map(|(&entity_type, &original_count)| {
            let scaled_losses = losses.get(&entity_type).copied().unwrap_or(0);

            if scaled_losses == 0 {
                // No losses - restore original count exactly
                Some((entity_type, original_count))
            } else {
                // Had losses - calculate remaining from original - scaled_losses
                let remaining_count = original_count.saturating_sub(scaled_losses);
                if remaining_count > 0 {
                    Some((entity_type, remaining_count))
                } else {
                    None
                }
            }
        })
        .collect()
}

/// Scale up a fleet by the given factor
fn upscale_fleet(fleet: &FleetComposition, factor: usize) -> FleetComposition {
    fleet
        .iter()
        .map(|(&entity_type, &count)| (entity_type, count * factor as u32))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use combat_types::Technology;
    use std::collections::HashMap;

    #[test]
    fn test_total_ships() {
        let mut fleet = HashMap::new();
        fleet.insert(204, 1000);
        fleet.insert(205, 500);

        assert_eq!(total_ships(&fleet), 1500);
    }

    #[test]
    fn test_should_downscale() {
        let tech = Technology {
            weapon: 0,
            shield: 0,
            armour: 0,
            ..Default::default()
        };

        // Small fleet - no downscaling
        let mut small_fleet = HashMap::new();
        small_fleet.insert(204, 1000);
        let small_party = PartyData {
            technology: tech,
            entities: small_fleet,
        };

        assert!(!should_downscale(&small_party, &small_party));

        // Large fleet - should downscale
        let mut large_fleet = HashMap::new();
        large_fleet.insert(204, 6_000_000);
        let large_party = PartyData {
            technology: tech,
            entities: large_fleet,
        };

        assert!(should_downscale(&large_party, &large_party));
    }

    #[test]
    fn test_downscale_fleet() {
        let mut fleet = HashMap::new();
        fleet.insert(204, 10000);
        fleet.insert(205, 5000);

        let downscaled = downscale_fleet(&fleet, 10);

        assert_eq!(downscaled.get(&204), Some(&1000));
        assert_eq!(downscaled.get(&205), Some(&500));
    }

    #[test]
    fn test_downscale_preserves_small_counts() {
        let mut fleet = HashMap::new();
        fleet.insert(204, 5); // Very small count

        let downscaled = downscale_fleet(&fleet, 10);

        // Should keep at least 1
        assert_eq!(downscaled.get(&204), Some(&1));
    }

    #[test]
    fn test_upscale_result() {
        use combat_types::CombatOutcome;

        let mut losses = HashMap::new();
        losses.insert(204, 100);

        let mut remaining = HashMap::new();
        remaining.insert(204, 900);

        let result = SimulationResult {
            outcome: CombatOutcome::AttackersWin,
            rounds: 5,
            attacker_losses: losses.clone(),
            defender_losses: HashMap::new(),
            attacker_remaining: remaining.clone(),
            defender_remaining: HashMap::new(),
            debris_field: combat_types::DebrisField {
                metal: 1000,
                crystal: 500,
            },
            loot: combat_types::PlanetResources {
                metal: 2000,
                crystal: 1000,
                deuterium: 500,
            },
            attacker_profit: 5000,
            defender_profit: -3000,
            round_details: None,
            round_compositions: None,
            round_compositions_by_slot: None,
            attacker_slots: None,
            defender_slots: None,
        };

        let upscaled = upscale_result(&result, 10);

        assert_eq!(upscaled.attacker_losses.get(&204), Some(&1000));
        assert_eq!(upscaled.attacker_remaining.get(&204), Some(&9000));
        assert_eq!(upscaled.rounds, 5); // Rounds don't scale

        // Verify economic fields scale correctly
        assert_eq!(upscaled.debris_field.metal, 10000);
        assert_eq!(upscaled.debris_field.crystal, 5000);
        assert_eq!(upscaled.loot.metal, 20000);
        assert_eq!(upscaled.loot.crystal, 10000);
        assert_eq!(upscaled.loot.deuterium, 5000);
        assert_eq!(upscaled.attacker_profit, 50000);
        assert_eq!(upscaled.defender_profit, -30000)
    }

    #[test]
    fn test_calculate_downscale_factor() {
        let tech = Technology {
            weapon: 0,
            shield: 0,
            armour: 0,
            ..Default::default()
        };

        // 5M ships - no downscaling (below 10M threshold)
        let mut fleet1 = HashMap::new();
        fleet1.insert(204, 2_500_000);
        let party1 = PartyData {
            technology: tech,
            entities: fleet1,
        };
        assert_eq!(calculate_downscale_factor(&party1, &party1), 1);

        // 20M ships - 10x downscaling (>10M threshold)
        let mut fleet2 = HashMap::new();
        fleet2.insert(204, 10_000_000);
        let party2 = PartyData {
            technology: tech,
            entities: fleet2,
        };
        assert_eq!(calculate_downscale_factor(&party2, &party2), 10);

        // 100M ships - 50x downscaling (>50M threshold)
        let mut fleet3 = HashMap::new();
        fleet3.insert(204, 50_000_000);
        let party3 = PartyData {
            technology: tech,
            entities: fleet3,
        };
        assert_eq!(calculate_downscale_factor(&party3, &party3), 50);

        // 250M ships - 100x downscaling (>100M threshold)
        let mut fleet4 = HashMap::new();
        fleet4.insert(204, 125_000_000);
        let party4 = PartyData {
            technology: tech,
            entities: fleet4,
        };
        assert_eq!(calculate_downscale_factor(&party4, &party4), 100);

        // Edge case: exactly 10M - should be 10x
        let mut fleet5 = HashMap::new();
        fleet5.insert(204, 5_000_001);
        let party5 = PartyData {
            technology: tech,
            entities: fleet5,
        };
        assert_eq!(calculate_downscale_factor(&party5, &party5), 10);
    }
}
