// clippy::implicit_hasher wants every public fn here generic over the map's
// hasher. The maps are `FleetComposition` and the entity table — type aliases
// with a fixed hasher, built by this crate and never by a caller — so the
// generic parameter would appear in six signatures to be instantiated one way.
#![allow(clippy::implicit_hasher)]

use combat_types::EntityStats;
use combat_types::{DebrisField, EntityType, FleetComposition, PlanetResources};
use std::collections::HashMap;

/// Calculate debris field from losses
/// Now supports separate debris percentages for fleet and defence
#[must_use]
pub fn calculate_debris(
    attacker_losses: &FleetComposition,
    defender_losses: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
    debris_percentage: f32,
) -> DebrisField {
    // Use default debris % for both fleet and defence (legacy behavior)
    calculate_debris_extended(
        attacker_losses,
        defender_losses,
        entity_db,
        debris_percentage as u8,
        0,
    )
}

/// Calculate debris field with separate fleet and defence percentages
#[must_use]
pub fn calculate_debris_extended(
    attacker_losses: &FleetComposition,
    defender_losses: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
    debris_fleet_pct: u8,
    debris_defence_pct: u8,
) -> DebrisField {
    let mut metal = 0u64;
    let mut crystal = 0u64;

    let fleet_factor = f32::from(debris_fleet_pct) / 100.0;
    let defence_factor = f32::from(debris_defence_pct) / 100.0;

    // Calculate debris from attacker losses (always ships)
    for (&entity_type, &count) in attacker_losses {
        if let Some(stats) = entity_db.get(&entity_type) {
            // Ships (< 400) go to debris at fleet rate
            if entity_type < 400 {
                metal += (stats.cost_metal as f32 * fleet_factor * count as f32) as u64;
                crystal += (stats.cost_crystal as f32 * fleet_factor * count as f32) as u64;
            }
        }
    }

    // Calculate debris from defender losses
    for (&entity_type, &count) in defender_losses {
        if let Some(stats) = entity_db.get(&entity_type) {
            if entity_type < 400 {
                // Ships go to debris at fleet rate
                metal += (stats.cost_metal as f32 * fleet_factor * count as f32) as u64;
                crystal += (stats.cost_crystal as f32 * fleet_factor * count as f32) as u64;
            } else if (400..500).contains(&entity_type) {
                // Defense (400-499) goes to debris at defence rate
                metal += (stats.cost_metal as f32 * defence_factor * count as f32) as u64;
                crystal += (stats.cost_crystal as f32 * defence_factor * count as f32) as u64;
            }
        }
    }

    DebrisField { metal, crystal }
}

/// Calculate loot from planet resources
#[must_use]
pub fn calculate_loot(planet_resources: &PlanetResources, cargo_capacity: u64) -> PlanetResources {
    // Default 50% plunder
    calculate_loot_extended(planet_resources, cargo_capacity, 50)
}

/// Calculate loot with custom plunder percentage (50%, 75%, or 100%)
#[must_use]
pub fn calculate_loot_extended(
    planet_resources: &PlanetResources,
    cargo_capacity: u64,
    plunder_percentage: u8,
) -> PlanetResources {
    let plunder_factor = f64::from(plunder_percentage) / 100.0;
    let max_metal = (planet_resources.metal as f64 * plunder_factor) as u64;
    let max_crystal = (planet_resources.crystal as f64 * plunder_factor) as u64;
    let max_deuterium = (planet_resources.deuterium as f64 * plunder_factor) as u64;

    let total_available = max_metal + max_crystal + max_deuterium;

    if total_available <= cargo_capacity {
        // Can take all available resources
        PlanetResources {
            metal: max_metal,
            crystal: max_crystal,
            deuterium: max_deuterium,
        }
    } else {
        // Distribute cargo capacity optimally
        // Priority: Metal > Crystal > Deuterium (standard OGame priority)
        let mut remaining = cargo_capacity;
        let mut loot = PlanetResources {
            metal: 0,
            crystal: 0,
            deuterium: 0,
        };

        // Fill metal first
        loot.metal = max_metal.min(remaining);
        remaining = remaining.saturating_sub(loot.metal);

        // Then crystal
        loot.crystal = max_crystal.min(remaining);
        remaining = remaining.saturating_sub(loot.crystal);

        // Finally deuterium
        loot.deuterium = max_deuterium.min(remaining);

        loot
    }
}

/// Calculate total cargo capacity of a fleet
#[must_use]
pub fn calculate_cargo_capacity(
    fleet: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
) -> u64 {
    let mut capacity = 0u64;

    for (&entity_type, &count) in fleet {
        if let Some(stats) = entity_db.get(&entity_type) {
            capacity += u64::from(stats.cargo_capacity) * u64::from(count);
        }
    }

    capacity
}

/// Calculate value of losses (metal + crystal + deuterium)
#[must_use]
pub fn calculate_losses_value(
    losses: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
) -> u64 {
    let mut value = 0u64;

    for (&entity_type, &count) in losses {
        if let Some(stats) = entity_db.get(&entity_type) {
            value += (u64::from(stats.cost_metal)
                + u64::from(stats.cost_crystal)
                + u64::from(stats.cost_deuterium))
                * u64::from(count);
        }
    }

    value
}

/// Calculate profit for attacker
/// Profit = Debris + Loot - Losses - Fuel Cost
#[must_use]
pub fn calculate_attacker_profit(
    debris: &DebrisField,
    loot: &PlanetResources,
    losses: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
) -> i64 {
    let gains = debris.total() + loot.total();
    let losses_value = calculate_losses_value(losses, entity_db);

    // For now, ignore fuel cost (would need distance calculation)
    // TODO: Add fuel cost when flight mechanics are implemented

    gains as i64 - losses_value as i64
}

/// Calculate profit for defender
/// Profit = Debris - Losses
#[must_use]
pub fn calculate_defender_profit(
    debris: &DebrisField,
    losses: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
) -> i64 {
    let gains = debris.total();
    let losses_value = calculate_losses_value(losses, entity_db);

    gains as i64 - losses_value as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use combat_types::entities::load_entity_stats;

    #[test]
    fn test_calculate_debris() {
        let entity_db = load_entity_stats();

        let mut attacker_losses = HashMap::new();
        attacker_losses.insert(204, 100); // 100 Light Fighters lost

        let defender_losses = HashMap::new();

        let debris = calculate_debris(&attacker_losses, &defender_losses, &entity_db, 30.0);

        // Light Fighter costs: 3000 metal, 1000 crystal
        // 100 fighters * 30% = 30 fighters worth
        // Expected: 90,000 metal, 30,000 crystal
        assert_eq!(debris.metal, 90000);
        assert_eq!(debris.crystal, 30000);
    }

    #[test]
    fn test_calculate_loot_full_capacity() {
        let resources = PlanetResources {
            metal: 100_000,
            crystal: 50000,
            deuterium: 25000,
        };

        // Enough cargo for all 50%
        let loot = calculate_loot(&resources, 100_000);

        assert_eq!(loot.metal, 50000);
        assert_eq!(loot.crystal, 25000);
        assert_eq!(loot.deuterium, 12500);
    }

    #[test]
    fn test_calculate_loot_limited_capacity() {
        let resources = PlanetResources {
            metal: 100_000,
            crystal: 50000,
            deuterium: 25000,
        };

        // Limited cargo - should prioritize metal
        let loot = calculate_loot(&resources, 60000);

        assert_eq!(loot.metal, 50000); // Full metal
        assert_eq!(loot.crystal, 10000); // Partial crystal
        assert_eq!(loot.deuterium, 0); // No room for deuterium
    }

    #[test]
    fn test_calculate_cargo_capacity() {
        let entity_db = load_entity_stats();

        let mut fleet = HashMap::new();
        fleet.insert(202, 10); // 10 Small Cargo (5000 each)
        fleet.insert(203, 5); // 5 Large Cargo (25000 each)

        let capacity = calculate_cargo_capacity(&fleet, &entity_db);

        // Expected: 10 * 5000 + 5 * 25000 = 50000 + 125000 = 175000
        assert_eq!(capacity, 175_000);
    }

    #[test]
    fn test_calculate_losses_value() {
        let entity_db = load_entity_stats();

        let mut losses = HashMap::new();
        losses.insert(204, 100); // 100 Light Fighters

        let value = calculate_losses_value(&losses, &entity_db);

        // Light Fighter: 3000 + 1000 + 0 = 4000 per ship
        // 100 * 4000 = 400,000
        assert_eq!(value, 400_000);
    }

    #[test]
    fn test_profit_calculations() {
        let entity_db = load_entity_stats();

        let debris = DebrisField {
            metal: 90000,
            crystal: 30000,
        };

        let loot = PlanetResources {
            metal: 50000,
            crystal: 25000,
            deuterium: 12500,
        };

        let mut losses = HashMap::new();
        losses.insert(204, 50); // 50 Light Fighters lost

        let attacker_profit = calculate_attacker_profit(&debris, &loot, &losses, &entity_db);

        // Gains: 120000 (debris) + 87500 (loot) = 207500
        // Losses: 50 * 4000 = 200000
        // Profit: 207500 - 200000 = 7500
        assert_eq!(attacker_profit, 7500);
    }

    #[test]
    fn test_reaper_debris() {
        let entity_db = load_entity_stats();

        // Verify Reaper (218) is in the database with correct costs
        let reaper = entity_db
            .get(&218)
            .expect("Reaper (218) should be in entity DB");
        assert_eq!(reaper.cost_metal, 85000, "Reaper metal cost");
        assert_eq!(reaper.cost_crystal, 55000, "Reaper crystal cost");

        // Simulate 10 Reapers lost
        let mut losses = HashMap::new();
        losses.insert(218, 10);

        let debris = calculate_debris(&losses, &HashMap::new(), &entity_db, 30.0);

        // 10 Reapers * 30% debris
        // Metal: 10 * 85000 * 0.30 = 255,000
        // Crystal: 10 * 55000 * 0.30 = 165,000
        assert_eq!(debris.metal, 255_000, "Reaper debris metal");
        assert_eq!(debris.crystal, 165_000, "Reaper debris crystal");
    }

    #[test]
    fn test_pathfinder_debris() {
        let entity_db = load_entity_stats();

        // Verify Pathfinder (219) is in the database with correct costs
        let pathfinder = entity_db
            .get(&219)
            .expect("Pathfinder (219) should be in entity DB");
        assert_eq!(pathfinder.cost_metal, 8000, "Pathfinder metal cost");
        assert_eq!(pathfinder.cost_crystal, 15000, "Pathfinder crystal cost");

        // Simulate 100 Pathfinders lost
        let mut losses = HashMap::new();
        losses.insert(219, 100);

        let debris = calculate_debris(&losses, &HashMap::new(), &entity_db, 30.0);

        // 100 Pathfinders * 30% debris
        // Metal: 100 * 8000 * 0.30 = 240,000
        // Crystal: 100 * 15000 * 0.30 = 450,000
        assert_eq!(debris.metal, 240_000, "Pathfinder debris metal");
        assert_eq!(debris.crystal, 450_000, "Pathfinder debris crystal");
    }
}
