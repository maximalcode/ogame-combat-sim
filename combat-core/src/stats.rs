use combat_types::{EntityStats, EntityType, Technology};
use std::collections::HashMap;

/// Calculate modified stats for an entity based on technology levels
pub struct ModifiedStats {
    pub weapon: u32,
    pub shield: f32,
    pub hull: f32,
}

impl ModifiedStats {
    pub fn calculate(base_stats: &EntityStats, tech: &Technology) -> Self {
        let base_weapon = base_stats.weapon as f32;
        let base_shield = base_stats.shield as f32;
        let base_armour = base_stats.armour as f32;

        // Technology bonus: +10% per level
        let weapon_modifier = 1.0 + (tech.weapon as f32 * 0.1);
        let shield_modifier = 1.0 + (tech.shield as f32 * 0.1);
        let armour_modifier = 1.0 + (tech.armour as f32 * 0.1);

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
    pub fn new(entity_stats: &HashMap<EntityType, EntityStats>, tech: &Technology) -> Self {
        let stats = entity_stats
            .iter()
            .map(|(&entity_type, base_stats)| {
                (entity_type, ModifiedStats::calculate(base_stats, tech))
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
    use combat_types::entities::load_entity_stats;

    #[test]
    fn test_stats_no_tech() {
        let entity_db = load_entity_stats();
        let light_fighter = entity_db.get(&204).unwrap();
        let tech = Technology::default();

        let modified = ModifiedStats::calculate(light_fighter, &tech);

        assert_eq!(modified.weapon, 50);
        assert_eq!(modified.shield, 10.0);
        assert_eq!(modified.hull, 400.0); // 4000 * 0.1
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

        let modified = ModifiedStats::calculate(light_fighter, &tech);

        // weapon: 50 * 2.0 = 100
        assert_eq!(modified.weapon, 100);
        // shield: 10 * 2.0 = 20
        assert_eq!(modified.shield, 20.0);
        // hull: (4000 * 2.0) * 0.1 = 800
        assert_eq!(modified.hull, 800.0);
    }

    #[test]
    fn test_stats_cache() {
        let entity_db = load_entity_stats();
        let tech = Technology {
            weapon: 5,
            shield: 5,
            armour: 5,
            ..Default::default()
        };

        let cache = StatsCache::new(&entity_db, &tech);

        let light_fighter_stats = cache.get(204).unwrap();
        assert_eq!(light_fighter_stats.weapon, 75); // 50 * 1.5
        assert_eq!(light_fighter_stats.shield, 15.0); // 10 * 1.5
        assert_eq!(light_fighter_stats.hull, 600.0); // (4000 * 1.5) * 0.1
    }
}
