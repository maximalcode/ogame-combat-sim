// clippy::implicit_hasher wants every public fn here generic over the map's
// hasher. The maps are `FleetComposition` and the entity table — type aliases
// with a fixed hasher, built by this crate and never by a caller — so the
// generic parameter would appear in six signatures to be instantiated one way.
#![allow(clippy::implicit_hasher)]

use combat_types::EntityStats;
use combat_types::{DebrisField, DebrisSettings, EntityType, FleetComposition, PlanetResources};
use std::collections::HashMap;

/// Entity ids below this are ships; `400..500` are defences. The two leave
/// debris at different rates, and in a standard universe defences leave none.
const FIRST_DEFENCE_ID: EntityType = 400;
const FIRST_NON_DEFENCE_ID: EntityType = 500;

/// Calculate the debris field left by a battle's losses.
///
/// Ships and defences are counted at their own percentages, and deuterium only
/// joins metal and crystal when the universe says so — all three come from
/// [`DebrisSettings`], which [`combat_types::CombatRequest::debris_settings`]
/// resolves. Only the defender's defences can contribute: an attacker cannot
/// bring a rocket launcher along.
#[must_use]
pub fn calculate_debris(
    attacker_losses: &FleetComposition,
    defender_losses: &FleetComposition,
    entity_db: &HashMap<EntityType, EntityStats>,
    settings: DebrisSettings,
) -> DebrisField {
    let fleet_factor = f32::from(settings.fleet_percentage) / 100.0;
    let defence_factor = f32::from(settings.defence_percentage) / 100.0;

    let mut field = DebrisField::default();

    // Attacker losses are always ships.
    for (&entity_type, &count) in attacker_losses {
        if entity_type < FIRST_DEFENCE_ID {
            add_wreck(
                &mut field,
                entity_db,
                entity_type,
                count,
                fleet_factor,
                settings.deuterium,
            );
        }
    }

    for (&entity_type, &count) in defender_losses {
        if entity_type < FIRST_DEFENCE_ID {
            add_wreck(
                &mut field,
                entity_db,
                entity_type,
                count,
                fleet_factor,
                settings.deuterium,
            );
        } else if entity_type < FIRST_NON_DEFENCE_ID {
            add_wreck(
                &mut field,
                entity_db,
                entity_type,
                count,
                defence_factor,
                settings.deuterium,
            );
        }
    }

    field
}

/// Add one entity type's share of a wreck to the field. Unknown ids contribute
/// nothing, which is how they have always been treated.
fn add_wreck(
    field: &mut DebrisField,
    entity_db: &HashMap<EntityType, EntityStats>,
    entity_type: EntityType,
    count: u32,
    factor: f32,
    include_deuterium: bool,
) {
    let Some(stats) = entity_db.get(&entity_type) else {
        return;
    };

    let share = |cost: u32| (cost as f32 * factor * count as f32) as u64;

    field.metal += share(stats.cost_metal);
    field.crystal += share(stats.cost_crystal);
    if include_deuterium {
        field.deuterium += share(stats.cost_deuterium);
    }
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

    /// A standard universe: 30% of ships, no defence debris, no deuterium.
    fn standard() -> DebrisSettings {
        DebrisSettings {
            fleet_percentage: 30,
            defence_percentage: 0,
            deuterium: false,
        }
    }

    #[test]
    fn test_calculate_debris() {
        let entity_db = load_entity_stats();

        let mut attacker_losses = HashMap::new();
        attacker_losses.insert(204, 100); // 100 Light Fighters lost

        let defender_losses = HashMap::new();

        let debris = calculate_debris(&attacker_losses, &defender_losses, &entity_db, standard());

        // Light Fighter costs: 3000 metal, 1000 crystal
        // 100 fighters * 30% = 30 fighters worth
        // Expected: 90,000 metal, 30,000 crystal
        assert_eq!(debris.metal, 90000);
        assert_eq!(debris.crystal, 30000);
    }

    /// The point of the whole exercise: one percentage applied to both ships
    /// and defences is wrong, and a universe that sets them apart must see them
    /// apart.
    #[test]
    fn ships_and_defences_leave_debris_at_their_own_rates() {
        let entity_db = load_entity_stats();

        let mut defender_losses = HashMap::new();
        defender_losses.insert(211, 10); // Bomber:      50000 / 25000 / 15000
        defender_losses.insert(404, 10); // Gauss cannon: 20000 / 15000 /  2000

        let debris = calculate_debris(
            &HashMap::new(),
            &defender_losses,
            &entity_db,
            DebrisSettings {
                fleet_percentage: 50,
                defence_percentage: 20,
                deuterium: false,
            },
        );

        // Ships at 50%:    10 * 50000 * 0.5 = 250_000 metal, 125_000 crystal
        // Defences at 20%: 10 * 20000 * 0.2 =  40_000 metal,  30_000 crystal
        assert_eq!(debris.metal, 290_000, "fleet and defence metal");
        assert_eq!(debris.crystal, 155_000, "fleet and defence crystal");
        assert_eq!(debris.deuterium, 0, "deuterium is off in this universe");
    }

    /// A single percentage would have produced the same figure for both halves;
    /// this pins the fact that the defence half can be switched off entirely
    /// while the fleet half keeps contributing.
    #[test]
    fn defences_leave_nothing_when_the_universe_says_zero() {
        let entity_db = load_entity_stats();

        let mut defender_losses = HashMap::new();
        defender_losses.insert(404, 10); // 10 Gauss cannons

        let debris = calculate_debris(&HashMap::new(), &defender_losses, &entity_db, standard());

        assert_eq!(debris.total(), 0);
    }

    /// Deuterium in debris fields, the per-universe option added in v9.2.0.
    #[test]
    fn deuterium_joins_the_field_when_the_universe_allows_it() {
        let entity_db = load_entity_stats();

        let mut defender_losses = HashMap::new();
        defender_losses.insert(211, 10); // Bomber:      15000 deuterium each
        defender_losses.insert(404, 10); // Gauss cannon: 2000 deuterium each

        let debris = calculate_debris(
            &HashMap::new(),
            &defender_losses,
            &entity_db,
            DebrisSettings {
                fleet_percentage: 50,
                defence_percentage: 20,
                deuterium: true,
            },
        );

        // 10 * 15000 * 0.5 = 75_000, plus 10 * 2000 * 0.2 = 4_000
        assert_eq!(debris.deuterium, 79_000);
        // The metal and crystal halves are untouched by the option.
        assert_eq!(debris.metal, 290_000);
        assert_eq!(debris.crystal, 155_000);
    }

    /// Deuterium is opt-in, so the same battle in a standard universe leaves
    /// none — and `total()` must not quietly start counting it.
    #[test]
    fn deuterium_stays_out_of_the_field_by_default() {
        let entity_db = load_entity_stats();

        let mut attacker_losses = HashMap::new();
        attacker_losses.insert(211, 10); // Bombers, which do cost deuterium

        let debris = calculate_debris(&attacker_losses, &HashMap::new(), &entity_db, standard());

        assert_eq!(debris.deuterium, 0);
        assert_eq!(debris.total(), debris.metal + debris.crystal);
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
            deuterium: 0,
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

        let debris = calculate_debris(&losses, &HashMap::new(), &entity_db, standard());

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

        let debris = calculate_debris(&losses, &HashMap::new(), &entity_db, standard());

        // 100 Pathfinders * 30% debris
        // Metal: 100 * 8000 * 0.30 = 240,000
        // Crystal: 100 * 15000 * 0.30 = 450,000
        assert_eq!(debris.metal, 240_000, "Pathfinder debris metal");
        assert_eq!(debris.crystal, 450_000, "Pathfinder debris crystal");
    }
}
