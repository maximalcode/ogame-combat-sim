//! Ship and defence statistics for `OGame`.
//!
//! These are base values. Weapons, shielding and armour technology scale them
//! by +10% per level, player and alliance classes are worth further levels of
//! the same, and lifeform research adds a per-ship-type percentage of the base
//! on top — all three are terms of one sum, applied in
//! `ModifiedStats::calculate`.
//!
//! The numbers here are the game's own published unit stats — build costs,
//! structural integrity, shield and weapon power, cargo capacity, base speed and
//! fuel consumption. They were transcribed from public sources and cross-checked
//! against `TrashSim` (<https://trashsim.universeview.be/> by Klaas), which
//! publishes the same table. No `TrashSim` code is used or derived here; the combat
//! mechanics in `combat-core` are an independent reimplementation.
//!
//! This project is an unofficial fan tool and is not affiliated with, endorsed
//! by, or sponsored by Gameforge. See the README for the full disclaimer.

use crate::{EntityStats, EntityType};
use std::collections::HashMap;
use std::sync::LazyLock;

/// The table, built once per process.
///
/// It is constant data, and it was previously rebuilt on every simulation —
/// roughly 27 `HashMap` inserts, each carrying two more `HashMap`s for the
/// rapid-fire rows, inside the hot loop.
static ENTITY_STATS: LazyLock<HashMap<EntityType, EntityStats>> = LazyLock::new(build_entity_stats);

/// Borrow the shared entity stats database.
///
/// Prefer this everywhere. [`load_entity_stats`] exists for the callers that
/// genuinely need an owned, mutable copy, and it pays for one.
#[must_use]
pub fn entity_stats() -> &'static HashMap<EntityType, EntityStats> {
    &ENTITY_STATS
}

/// Load an owned copy of the entity stats database.
///
/// A clone of the shared table. Use [`entity_stats`] unless you need to mutate
/// the result — several tests build a doctored table this way.
#[must_use]
pub fn load_entity_stats() -> HashMap<EntityType, EntityStats> {
    ENTITY_STATS.clone()
}

fn build_entity_stats() -> HashMap<EntityType, EntityStats> {
    let mut stats = HashMap::new();

    // Ships
    stats.insert(202, small_cargo());
    stats.insert(203, large_cargo());
    stats.insert(204, light_fighter());
    stats.insert(205, heavy_fighter());
    stats.insert(206, cruiser());
    stats.insert(207, battleship());
    stats.insert(208, colony_ship());
    stats.insert(209, recycler());
    stats.insert(210, espionage_probe());
    stats.insert(211, bomber());
    stats.insert(212, solar_satellite());
    stats.insert(213, destroyer());
    stats.insert(214, deathstar());
    stats.insert(215, battlecruiser());
    stats.insert(217, crawler());
    stats.insert(218, reaper());
    stats.insert(219, pathfinder());

    // Defenses
    stats.insert(401, rocket_launcher());
    stats.insert(402, light_laser());
    stats.insert(403, heavy_laser());
    stats.insert(404, gauss_cannon());
    stats.insert(405, ion_cannon());
    stats.insert(406, plasma_turret());
    stats.insert(407, small_shield_dome());
    stats.insert(408, large_shield_dome());

    // Missiles
    stats.insert(502, anti_ballistic_missile());
    stats.insert(503, interplanetary_missile());

    stats
}

// Helper macro for creating rapid fire maps
macro_rules! rf_map {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = HashMap::new();
        $(map.insert($key, $val);)*
        map
    }};
}

// Ships

fn small_cargo() -> EntityStats {
    EntityStats {
        entity_type: 202,
        weapon: 5,
        shield: 10,
        armour: 4000,
        rapid_fire_from: rf_map!(215 => 3, 205 => 3, 214 => 250),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5),
        cost_metal: 2000,
        cost_crystal: 2000,
        cost_deuterium: 0,
        cargo_capacity: 5000,
        base_speed: 5000,
        fuel_consumption: 10,
    }
}

fn large_cargo() -> EntityStats {
    EntityStats {
        entity_type: 203,
        weapon: 5,
        shield: 25,
        armour: 12000,
        rapid_fire_from: rf_map!(215 => 3, 214 => 250),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5),
        cost_metal: 6000,
        cost_crystal: 6000,
        cost_deuterium: 0,
        cargo_capacity: 25000,
        base_speed: 7500,
        fuel_consumption: 50,
    }
}

fn light_fighter() -> EntityStats {
    EntityStats {
        entity_type: 204,
        weapon: 50,
        shield: 10,
        armour: 4000,
        rapid_fire_from: rf_map!(206 => 6, 214 => 200, 219 => 3),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5),
        cost_metal: 3000,
        cost_crystal: 1000,
        cost_deuterium: 0,
        cargo_capacity: 50,
        base_speed: 12500,
        fuel_consumption: 20,
    }
}

fn heavy_fighter() -> EntityStats {
    EntityStats {
        entity_type: 205,
        weapon: 150,
        shield: 25,
        armour: 10000,
        rapid_fire_from: rf_map!(215 => 4, 214 => 100, 219 => 2),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5, 202 => 3),
        cost_metal: 6000,
        cost_crystal: 4000,
        cost_deuterium: 0,
        cargo_capacity: 100,
        base_speed: 10000,
        fuel_consumption: 75,
    }
}

fn cruiser() -> EntityStats {
    EntityStats {
        entity_type: 206,
        weapon: 400,
        shield: 50,
        armour: 27000,
        rapid_fire_from: rf_map!(215 => 4, 214 => 33, 219 => 3),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5, 204 => 6, 401 => 10),
        cost_metal: 20000,
        cost_crystal: 7000,
        cost_deuterium: 2000,
        cargo_capacity: 800,
        base_speed: 15000,
        fuel_consumption: 300,
    }
}

fn battleship() -> EntityStats {
    EntityStats {
        entity_type: 207,
        weapon: 1000,
        shield: 200,
        armour: 60000,
        rapid_fire_from: rf_map!(215 => 7, 214 => 30, 218 => 7),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5, 219 => 5),
        cost_metal: 45000,
        cost_crystal: 15000,
        cost_deuterium: 0,
        cargo_capacity: 1500,
        base_speed: 10000,
        fuel_consumption: 500,
    }
}

fn colony_ship() -> EntityStats {
    EntityStats {
        entity_type: 208,
        weapon: 50,
        shield: 100,
        armour: 30000,
        rapid_fire_from: rf_map!(214 => 250),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5),
        cost_metal: 10000,
        cost_crystal: 20000,
        cost_deuterium: 10000,
        cargo_capacity: 7500,
        base_speed: 2500,
        fuel_consumption: 1000,
    }
}

fn recycler() -> EntityStats {
    EntityStats {
        entity_type: 209,
        weapon: 1,
        shield: 10,
        armour: 16000,
        rapid_fire_from: rf_map!(214 => 250),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5),
        cost_metal: 10000,
        cost_crystal: 6000,
        cost_deuterium: 2000,
        cargo_capacity: 20000,
        base_speed: 2000,
        fuel_consumption: 300,
    }
}

fn espionage_probe() -> EntityStats {
    EntityStats {
        entity_type: 210,
        weapon: 0,
        shield: 0,
        armour: 1000,
        rapid_fire_from: rf_map!(
            215 => 5, 213 => 5, 211 => 5, 209 => 5, 208 => 5,
            207 => 5, 206 => 5, 205 => 5, 204 => 5, 203 => 5,
            214 => 250, 218 => 5, 219 => 5, 202 => 5
        ),
        rapid_fire_against: HashMap::new(),
        cost_metal: 0,
        cost_crystal: 1000,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 100_000_000,
        fuel_consumption: 1,
    }
}

fn bomber() -> EntityStats {
    EntityStats {
        entity_type: 211,
        weapon: 1000,
        shield: 500,
        armour: 75000,
        rapid_fire_from: rf_map!(214 => 25, 218 => 4),
        rapid_fire_against: rf_map!(
            210 => 5, 212 => 5, 217 => 5,
            401 => 20, 402 => 20, 403 => 10, 404 => 5, 405 => 10, 406 => 5
        ),
        cost_metal: 50000,
        cost_crystal: 25000,
        cost_deuterium: 15000,
        cargo_capacity: 500,
        base_speed: 4000,
        fuel_consumption: 700,
    }
}

fn solar_satellite() -> EntityStats {
    EntityStats {
        entity_type: 212,
        weapon: 1,
        shield: 1,
        armour: 2000,
        rapid_fire_from: rf_map!(
            215 => 5, 213 => 5, 211 => 5, 209 => 5, 208 => 5,
            207 => 5, 206 => 5, 205 => 5, 204 => 5, 203 => 5,
            214 => 250, 218 => 5, 219 => 5, 202 => 5
        ),
        rapid_fire_against: HashMap::new(),
        cost_metal: 0,
        cost_crystal: 2000,
        cost_deuterium: 500,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn destroyer() -> EntityStats {
    EntityStats {
        entity_type: 213,
        weapon: 2000,
        shield: 500,
        armour: 110_000,
        rapid_fire_from: rf_map!(214 => 5, 218 => 3),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5, 402 => 10, 215 => 2),
        cost_metal: 60000,
        cost_crystal: 50000,
        cost_deuterium: 15000,
        cargo_capacity: 2000,
        base_speed: 5000,
        fuel_consumption: 1000,
    }
}

fn deathstar() -> EntityStats {
    EntityStats {
        entity_type: 214,
        weapon: 200_000,
        shield: 50000,
        armour: 9_000_000,
        rapid_fire_from: HashMap::new(),
        rapid_fire_against: rf_map!(
            202 => 250, 203 => 250, 204 => 200, 205 => 100, 206 => 33,
            207 => 30, 208 => 250, 209 => 250, 210 => 250, 212 => 250,
            211 => 25, 213 => 5, 401 => 200, 402 => 200, 403 => 100,
            404 => 50, 405 => 100, 215 => 15, 217 => 250, 218 => 10, 219 => 30
        ),
        cost_metal: 5_000_000,
        cost_crystal: 4_000_000,
        cost_deuterium: 1_000_000,
        cargo_capacity: 1_000_000,
        base_speed: 100,
        fuel_consumption: 1,
    }
}

fn battlecruiser() -> EntityStats {
    EntityStats {
        entity_type: 215,
        weapon: 700,
        shield: 400,
        armour: 70000,
        rapid_fire_from: rf_map!(213 => 2, 214 => 15),
        rapid_fire_against: rf_map!(
            210 => 5, 212 => 5, 217 => 5, 202 => 3, 203 => 3,
            205 => 4, 206 => 4, 207 => 7
        ),
        cost_metal: 30000,
        cost_crystal: 40000,
        cost_deuterium: 15000,
        cargo_capacity: 750,
        base_speed: 10000,
        fuel_consumption: 250,
    }
}

fn crawler() -> EntityStats {
    EntityStats {
        entity_type: 217,
        weapon: 1,
        shield: 1,
        armour: 4000,
        rapid_fire_from: rf_map!(
            215 => 5, 213 => 5, 211 => 5, 209 => 5, 208 => 5,
            207 => 5, 206 => 5, 205 => 5, 204 => 5, 203 => 5,
            214 => 250, 202 => 5, 219 => 5, 218 => 5
        ),
        rapid_fire_against: HashMap::new(),
        cost_metal: 2000,
        cost_crystal: 2000,
        cost_deuterium: 1000,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn reaper() -> EntityStats {
    EntityStats {
        entity_type: 218,
        weapon: 2800,
        shield: 700,
        armour: 140_000,
        rapid_fire_from: rf_map!(405 => 2, 214 => 10),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5, 207 => 7, 211 => 4, 213 => 3),
        cost_metal: 85000,
        cost_crystal: 55000,
        cost_deuterium: 20000,
        cargo_capacity: 10000,
        base_speed: 7000,
        fuel_consumption: 1100,
    }
}

fn pathfinder() -> EntityStats {
    EntityStats {
        entity_type: 219,
        weapon: 200,
        shield: 100,
        armour: 23000,
        rapid_fire_from: rf_map!(207 => 5, 214 => 30),
        rapid_fire_against: rf_map!(210 => 5, 212 => 5, 217 => 5, 204 => 3, 205 => 2, 206 => 3),
        cost_metal: 8000,
        cost_crystal: 15000,
        cost_deuterium: 8000,
        cargo_capacity: 10000,
        base_speed: 12000,
        fuel_consumption: 300,
    }
}

// Defenses

fn rocket_launcher() -> EntityStats {
    EntityStats {
        entity_type: 401,
        weapon: 80,
        shield: 20,
        armour: 2000,
        rapid_fire_from: rf_map!(211 => 20, 206 => 10, 214 => 200),
        rapid_fire_against: HashMap::new(),
        cost_metal: 2000,
        cost_crystal: 0,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn light_laser() -> EntityStats {
    EntityStats {
        entity_type: 402,
        weapon: 100,
        shield: 25,
        armour: 2000,
        rapid_fire_from: rf_map!(213 => 10, 211 => 20, 214 => 200),
        rapid_fire_against: HashMap::new(),
        cost_metal: 1500,
        cost_crystal: 500,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn heavy_laser() -> EntityStats {
    EntityStats {
        entity_type: 403,
        weapon: 250,
        shield: 100,
        armour: 8000,
        rapid_fire_from: rf_map!(211 => 10, 214 => 100),
        rapid_fire_against: HashMap::new(),
        cost_metal: 6000,
        cost_crystal: 2000,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn gauss_cannon() -> EntityStats {
    EntityStats {
        entity_type: 404,
        weapon: 1100,
        shield: 200,
        armour: 35000,
        rapid_fire_from: rf_map!(211 => 5, 214 => 50),
        rapid_fire_against: HashMap::new(),
        cost_metal: 20000,
        cost_crystal: 15000,
        cost_deuterium: 2000,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn ion_cannon() -> EntityStats {
    EntityStats {
        entity_type: 405,
        weapon: 150,
        shield: 500,
        armour: 8000,
        rapid_fire_from: rf_map!(211 => 10, 214 => 100),
        rapid_fire_against: rf_map!(218 => 2),
        cost_metal: 5000,
        cost_crystal: 3000,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn plasma_turret() -> EntityStats {
    EntityStats {
        entity_type: 406,
        weapon: 3000,
        shield: 300,
        armour: 100_000,
        rapid_fire_from: rf_map!(211 => 5),
        rapid_fire_against: HashMap::new(),
        cost_metal: 50000,
        cost_crystal: 50000,
        cost_deuterium: 30000,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn small_shield_dome() -> EntityStats {
    EntityStats {
        entity_type: 407,
        weapon: 1,
        shield: 2000,
        armour: 20000,
        rapid_fire_from: HashMap::new(),
        rapid_fire_against: HashMap::new(),
        cost_metal: 10000,
        cost_crystal: 10000,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn large_shield_dome() -> EntityStats {
    EntityStats {
        entity_type: 408,
        weapon: 1,
        shield: 10000,
        armour: 100_000,
        rapid_fire_from: HashMap::new(),
        rapid_fire_against: HashMap::new(),
        cost_metal: 50000,
        cost_crystal: 50000,
        cost_deuterium: 0,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

// Missiles

fn anti_ballistic_missile() -> EntityStats {
    EntityStats {
        entity_type: 502,
        weapon: 1,
        shield: 1,
        armour: 8000,
        rapid_fire_from: HashMap::new(),
        rapid_fire_against: HashMap::new(),
        cost_metal: 8000,
        cost_crystal: 0,
        cost_deuterium: 2000,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

fn interplanetary_missile() -> EntityStats {
    EntityStats {
        entity_type: 503,
        weapon: 12000,
        shield: 1,
        armour: 15000,
        rapid_fire_from: HashMap::new(),
        rapid_fire_against: HashMap::new(),
        cost_metal: 12500,
        cost_crystal: 2500,
        cost_deuterium: 10000,
        cargo_capacity: 0,
        base_speed: 0,
        fuel_consumption: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::entity_stats;
    use std::collections::HashMap;

    #[test]
    fn deathstar_rapid_fire_table_matches_ogame_v13() {
        let expected = HashMap::from([
            (202, 250),
            (203, 250),
            (204, 200),
            (205, 100),
            (206, 33),
            (207, 30),
            (208, 250),
            (209, 250),
            (210, 250),
            (211, 25),
            (212, 250),
            (213, 5),
            (215, 15),
            (217, 250),
            (218, 10),
            (219, 30),
            (401, 200),
            (402, 200),
            (403, 100),
            (404, 50),
            (405, 100),
        ]);

        assert_eq!(entity_stats()[&214].rapid_fire_against, expected);
    }

    #[test]
    fn reaper_rapid_fire_table_remains_unchanged() {
        let reaper = &entity_stats()[&218];

        assert_eq!(reaper.rapid_fire_from, HashMap::from([(405, 2), (214, 10)]));
        assert_eq!(
            reaper.rapid_fire_against,
            HashMap::from([(210, 5), (212, 5), (217, 5), (207, 7), (211, 4), (213, 3),])
        );
    }

    #[test]
    fn rapid_fire_tables_are_reciprocal() {
        let stats = entity_stats();

        for (&attacker, attacker_stats) in stats {
            for (&target, &multiplier) in &attacker_stats.rapid_fire_against {
                let target_stats = stats
                    .get(&target)
                    .unwrap_or_else(|| panic!("rapid-fire target {target} is not in entity stats"));
                assert_eq!(
                    target_stats.rapid_fire_from.get(&attacker),
                    Some(&multiplier),
                    "rapid-fire relationship {attacker} -> {target} is not reciprocal"
                );
            }
        }

        for (&target, target_stats) in stats {
            for (&attacker, &multiplier) in &target_stats.rapid_fire_from {
                let attacker_stats = stats.get(&attacker).unwrap_or_else(|| {
                    panic!("rapid-fire source {attacker} is not in entity stats")
                });
                assert_eq!(
                    attacker_stats.rapid_fire_against.get(&target),
                    Some(&multiplier),
                    "rapid-fire relationship {attacker} -> {target} is not reciprocal"
                );
            }
        }
    }
}
