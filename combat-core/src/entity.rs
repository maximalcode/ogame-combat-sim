use combat_types::EntityType;

/// A single combat entity instance
#[derive(Debug, Clone)]
pub struct Entity {
    // clippy::struct_field_names wants this not to repeat the struct name. It
    // is named for the domain type it holds, and `EntityType` is the name the
    // whole workspace uses for "which ship or defence is this".
    #[allow(clippy::struct_field_names)]
    pub entity_type: EntityType,
    pub is_alive: bool,
    pub current_shield: f32,
    pub current_hull: f32,
    pub max_shield: f32,
    pub max_hull: f32,
    pub weapon_power: u32,
    /// Optional slot identifier (0 = merged/unknown)
    pub slot_id: u8,
}

impl Entity {
    pub fn new(entity_type: EntityType, weapon_power: u32, max_shield: f32, max_hull: f32) -> Self {
        Self::new_with_slot(entity_type, weapon_power, max_shield, max_hull, 0)
    }

    /// Create entity with an explicit slot id
    pub fn new_with_slot(
        entity_type: EntityType,
        weapon_power: u32,
        max_shield: f32,
        max_hull: f32,
        slot_id: u8,
    ) -> Self {
        Self {
            entity_type,
            is_alive: true,
            current_shield: max_shield,
            current_hull: max_hull,
            max_shield,
            max_hull,
            weapon_power,
            slot_id,
        }
    }

    /// Reset shields to full (between combat rounds)
    pub fn regenerate_shield(&mut self) {
        if self.is_alive {
            self.current_shield = self.max_shield;
        }
    }

    /// Mark entity as destroyed
    pub fn destroy(&mut self) {
        self.is_alive = false;
    }

    /// Check if entity should explode due to hull damage
    /// When hull is below 70%, there's a probability of explosion
    pub fn check_explosion(&mut self, rng: &mut impl rand::Rng) -> bool {
        if !self.is_alive {
            return false;
        }

        let hull_percentage = self.current_hull / self.max_hull;

        // Only check explosion if hull is below 70%
        if hull_percentage <= 0.7 {
            // Explosion chance = 1 - (current_hull / max_hull)
            let explosion_chance = 1.0 - hull_percentage;

            if rng.random::<f32>() < explosion_chance {
                self.destroy();
                return true;
            }
        }

        false
    }
}
