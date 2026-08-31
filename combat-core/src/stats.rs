use combat_types::{EntityStats, EntityType, LifeformBonus, PartyData, Technology};
use std::collections::HashMap;

/// Calculate modified stats for an entity based on technology levels
pub struct ModifiedStats {
    pub weapon: u32,
    pub shield: f32,
    pub hull: f32,
}

impl ModifiedStats {
    /// Both modifiers are terms of one sum, applied to the base stat once.
    ///
    /// Technology is `+10%` per level and a lifeform bonus is a percentage of
    /// the same base, and `OGame` adds them rather than compounding them: a
    /// Destroyer (base weapon 2000) at Weapons 25 with `+50%` from lifeforms
    /// fires at `2000 * (1 + 2.5 + 0.5)` = 8000, which is the official forum's
    /// worked example. Scaling twice would give 10,500.
    ///
    /// Player and alliance classes are deliberately **not** here. They are
    /// worth levels, and arrive already folded into `tech` by
    /// [`Technology::effective_levels`] — a General with Weapons 10 reaches
    /// this function as Weapons 12.
    pub fn calculate(base_stats: &EntityStats, tech: &Technology, lifeform: LifeformBonus) -> Self {
        let base_weapon = base_stats.weapon as f32;
        let base_shield = base_stats.shield as f32;
        let base_armour = base_stats.armour as f32;

        // Technology bonus: +10% per level, plus the lifeform percentage
        let weapon_modifier = 1.0 + (f32::from(tech.weapon) * 0.1) + (lifeform.weapon / 100.0);
        let shield_modifier = 1.0 + (f32::from(tech.shield) * 0.1) + (lifeform.shield / 100.0);
        let armour_modifier = 1.0 + (f32::from(tech.armour) * 0.1) + (lifeform.armour / 100.0);

        let modified_weapon = (base_weapon * weapon_modifier).floor() as u32;
        let modified_shield = (base_shield * shield_modifier).floor();
        // Hull points = armour * 0.1
        let modified_hull = ((base_armour * armour_modifier) * 0.1).floor();

        Self {
            weapon: modified_weapon,
            shield: modified_shield,
            hull: modified_hull,
        }
    }
}

/// Precomputed stats for all entity types for a given technology level
pub struct StatsCache {
    stats: HashMap<EntityType, ModifiedStats>,
}

impl StatsCache {
    /// Built from a whole party rather than from its technology alone.
    ///
    /// Technology and lifeform bonuses are two halves of one side's stat
    /// modifiers, and taking them as separate arguments would let a caller pair
    /// one side's levels with the other side's lifeforms — a bug no test of
    /// either half would catch.
    pub fn new(entity_stats: &HashMap<EntityType, EntityStats>, party: &PartyData) -> Self {
        let stats = entity_stats
            .iter()
            .map(|(&entity_type, base_stats)| {
                (
                    entity_type,
                    ModifiedStats::calculate(
                        base_stats,
                        &party.technology,
                        party.lifeform.get(entity_type),
                    ),
                )
            })
            .collect();

        Self { stats }
    }

    pub fn get(&self, entity_type: EntityType) -> Option<&ModifiedStats> {
        self.stats.get(&entity_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Shield and hull are f32 and come out of a chain of multiplications, so
    // `assert_eq!` is asserting bit equality on a computed float. The values
    // below happen to be exactly representable today; a relative comparison
    // says what the test actually means and does not depend on that luck.
    use approx::assert_relative_eq;
    use combat_types::entities::load_entity_stats;
    use combat_types::{LifeformBonuses, PartyData};

    #[test]
    fn test_stats_no_tech() {
        let entity_db = load_entity_stats();
        let light_fighter = entity_db.get(&204).unwrap();
        let tech = Technology::default();

        let modified = ModifiedStats::calculate(light_fighter, &tech, LifeformBonus::default());

        assert_eq!(modified.weapon, 50);
        assert_relative_eq!(modified.shield, 10.0);
        assert_relative_eq!(modified.hull, 400.0); // 4000 * 0.1
    }

    #[test]
    fn test_stats_with_tech() {
        let entity_db = load_entity_stats();
        let light_fighter = entity_db.get(&204).unwrap();
        let tech = Technology {
            weapon: 10,
            shield: 10,
            armour: 10,
            ..Default::default()
        };

        let modified = ModifiedStats::calculate(light_fighter, &tech, LifeformBonus::default());

        // weapon: 50 * 2.0 = 100
        assert_eq!(modified.weapon, 100);
        // shield: 10 * 2.0 = 20
        assert_relative_eq!(modified.shield, 20.0);
        // hull: (4000 * 2.0) * 0.1 = 800
        assert_relative_eq!(modified.hull, 800.0);
    }

    #[test]
    fn test_stats_cache() {
        let entity_db = load_entity_stats();
        let party = PartyData {
            technology: Technology {
                weapon: 5,
                shield: 5,
                armour: 5,
                ..Default::default()
            },
            ..Default::default()
        };

        let cache = StatsCache::new(&entity_db, &party);

        let light_fighter_stats = cache.get(204).unwrap();
        assert_eq!(light_fighter_stats.weapon, 75); // 50 * 1.5
        assert_relative_eq!(light_fighter_stats.shield, 15.0); // 10 * 1.5
        assert_relative_eq!(light_fighter_stats.hull, 600.0); // (4000 * 1.5) * 0.1
    }

    /// The worked example from the official board, and the reason the two terms
    /// add rather than compound: a Destroyer at Weapons 25 with a `+50%`
    /// lifeform bonus fires at 8000. Compounding would give 10,500.
    #[test]
    fn the_destroyer_worked_example_comes_out_at_eight_thousand() {
        let entity_db = load_entity_stats();
        let destroyer = entity_db.get(&213).unwrap();
        let tech = Technology {
            weapon: 25,
            ..Default::default()
        };

        let modified = ModifiedStats::calculate(
            destroyer,
            &tech,
            LifeformBonus {
                weapon: 50.0,
                ..Default::default()
            },
        );

        assert_eq!(modified.weapon, 8000);
    }

    /// Hull, shield and firepower take their bonuses independently — one
    /// research may buff all three, but nothing in the type ties them together.
    #[test]
    fn each_stat_takes_its_own_lifeform_bonus() {
        let entity_db = load_entity_stats();
        let light_fighter = entity_db.get(&204).unwrap();

        let modified = ModifiedStats::calculate(
            light_fighter,
            &Technology::default(),
            LifeformBonus {
                shield: 50.0,
                ..Default::default()
            },
        );

        // Only the shield moves: 10 * 1.5 = 15, weapon and hull as they were.
        assert_eq!(modified.weapon, 50);
        assert_relative_eq!(modified.shield, 15.0);
        assert_relative_eq!(modified.hull, 400.0);
    }

    /// The bonus is per ship type, so a cache built for a fleet of two must
    /// move one of them and leave the other exactly where it was.
    #[test]
    fn the_cache_applies_a_bonus_to_only_the_ship_it_names() {
        let entity_db = load_entity_stats();
        let party = PartyData {
            technology: Technology {
                weapon: 10,
                shield: 10,
                armour: 10,
                ..Default::default()
            },
            lifeform: LifeformBonuses::from_iter([(204, LifeformBonus::uniform(50.0))]),
            ..Default::default()
        };

        let cache = StatsCache::new(&entity_db, &party);

        // Light Fighter: 50 * (1 + 1.0 + 0.5) = 125.
        assert_eq!(cache.get(204).unwrap().weapon, 125);
        // Cruiser, unnamed by the bonus: 400 * (1 + 1.0) = 800.
        assert_eq!(cache.get(206).unwrap().weapon, 800);
    }
}
