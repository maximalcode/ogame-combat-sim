use crate::entity::Entity;
use crate::instant::InstantCalculation;
use crate::stats::StatsCache;
use combat_types::{
    EntityStats, EntityType, FleetComposition, PartyData, RoundComposition, RoundDetails,
    SlotResult,
};
use rand::Rng;
use std::collections::HashMap;

const MAX_ROUNDS: u8 = 6;

#[derive(Default, Clone, Debug)]
struct FireStats {
    shots: u64,
    hull_damage: u64,
    shield_damage: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use combat_types::{PartyData, Technology};
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    #[test]
    fn compositions_flag_no_effect() {
        // Build small deterministic scenario
        let combat = Combat::new();

        let mut a_entities: FleetComposition = HashMap::new();
        a_entities.insert(206, 100); // 100 Cruisers

        let mut d_entities: FleetComposition = HashMap::new();
        d_entities.insert(204, 2000); // 2000 Light Fighters

        let attacker = PartyData {
            technology: Technology {
                weapon: 10,
                shield: 10,
                armour: 10,
                ..Default::default()
            },
            entities: a_entities,
            ..Default::default()
        };
        let defender = PartyData {
            technology: Technology {
                weapon: 10,
                shield: 10,
                armour: 10,
                ..Default::default()
            },
            entities: d_entities,
            ..Default::default()
        };

        // Use identical RNG seeds for the two runs
        let mut rng_no = SmallRng::seed_from_u64(42);
        let mut rng_yes = SmallRng::seed_from_u64(42);

        let r_no = combat.simulate_single(&attacker, &defender, true, false, &mut rng_no);
        let r_yes = combat.simulate_single(&attacker, &defender, true, true, &mut rng_yes);

        // Assert core outcomes are identical
        assert_eq!(r_no.outcome, r_yes.outcome, "Outcome must be identical");
        assert_eq!(r_no.rounds, r_yes.rounds, "Rounds must be identical");
        assert_eq!(
            r_no.attacker_losses, r_yes.attacker_losses,
            "Attacker losses must be identical"
        );
        assert_eq!(
            r_no.defender_losses, r_yes.defender_losses,
            "Defender losses must be identical"
        );
        assert_eq!(
            r_no.attacker_remaining, r_yes.attacker_remaining,
            "Attacker remaining must be identical"
        );
        assert_eq!(
            r_no.defender_remaining, r_yes.defender_remaining,
            "Defender remaining must be identical"
        );

        // Round details should also match exactly
        assert_eq!(
            r_no.round_details.as_ref().unwrap().len(),
            r_yes.round_details.as_ref().unwrap().len(),
            "Round details length must match"
        );
        for (a, b) in r_no
            .round_details
            .as_ref()
            .unwrap()
            .iter()
            .zip(r_yes.round_details.as_ref().unwrap().iter())
        {
            assert_eq!(a.round_number, b.round_number);
            assert_eq!(a.attackers_start, b.attackers_start);
            assert_eq!(a.defenders_start, b.defenders_start);
            assert_eq!(a.attackers_destroyed, b.attackers_destroyed);
            assert_eq!(a.defenders_destroyed, b.defenders_destroyed);
            assert_eq!(a.attackers_end, b.attackers_end);
            assert_eq!(a.defenders_end, b.defenders_end);
            assert_eq!(a.attacker_damage, b.attacker_damage);
            assert_eq!(a.defender_damage, b.defender_damage);
            assert_eq!(a.attacker_shots, b.attacker_shots);
            assert_eq!(a.defender_shots, b.defender_shots);
            assert_eq!(a.attacker_shield_damage, b.attacker_shield_damage);
            assert_eq!(a.defender_shield_damage, b.defender_shield_damage);
        }
    }
}

/// Combat party (attackers or defenders)
pub struct Party {
    pub entities: Vec<Entity>,
    pub rapid_fire_map: HashMap<EntityType, HashMap<EntityType, u16>>,
}

impl Party {
    pub fn new(
        party_data: &PartyData,
        entity_db: &HashMap<EntityType, EntityStats>,
        stats_cache: &StatsCache,
    ) -> Self {
        // Pre-calculate total size for allocation
        let total_count: u32 = party_data.entities.values().sum();
        let mut entities = Vec::with_capacity(total_count as usize);
        let mut rapid_fire_map = HashMap::with_capacity(party_data.entities.len());

        for (&entity_type, &count) in &party_data.entities {
            if count == 0 {
                continue;
            }

            if let Some(base_stats) = entity_db.get(&entity_type) {
                if let Some(modified_stats) = stats_cache.get(entity_type) {
                    // Store rapid fire info for this entity type
                    if !base_stats.rapid_fire_against.is_empty() {
                        rapid_fire_map.insert(entity_type, base_stats.rapid_fire_against.clone());
                    }

                    // Create entities - use extend for better performance
                    entities.extend((0..count).map(|_| {
                        Entity::new(
                            entity_type,
                            modified_stats.weapon,
                            modified_stats.shield,
                            modified_stats.hull,
                        )
                    }));
                }
            }
        }

        Self {
            entities,
            rapid_fire_map,
        }
    }

    pub fn remaining_count(&self) -> usize {
        self.entities.len()
    }

    fn count_by_type_alive(&self) -> FleetComposition {
        let mut counts: FleetComposition = HashMap::new();
        for e in &self.entities {
            if e.is_alive {
                *counts.entry(e.entity_type).or_insert(0) += 1;
            }
        }
        counts
    }

    fn shoot_at(
        &mut self,
        enemy: &mut Party,
        use_rapid_fire: bool,
        rng: &mut impl Rng,
        stats: &mut FireStats,
    ) {
        if enemy.entities.is_empty() {
            return;
        }

        let enemy_count = enemy.entities.len();

        // Pre-generate random numbers for better performance
        let mut random_targets = Vec::with_capacity(self.entities.len() * 2);
        let mut random_rf_checks = Vec::with_capacity(self.entities.len() * 2);

        for _ in 0..(self.entities.len() * 2) {
            random_targets.push(rng.random_range(0..enemy_count));
            random_rf_checks.push(rng.random::<f32>());
        }

        let mut rng_idx = 0;

        // Iterate through ALL entities - even dead ones should shoot in this round
        // They will be removed at the end of the round
        for i in 0..self.entities.len() {
            let entity = &self.entities[i];
            // Don't skip dead entities - they should still shoot if they were alive at round start
            // The entity removal happens at END of round, after both sides shoot

            let attacker_type = entity.entity_type;
            let weapon_power = entity.weapon_power;

            // Shoot at least once
            loop {
                // O(1) target selection - alive entities are at front of array
                let current_enemy_count = enemy.entities.len();
                if current_enemy_count == 0 {
                    break;
                }

                // Use pre-generated random number
                let target_idx = if rng_idx < random_targets.len() {
                    random_targets[rng_idx] % current_enemy_count
                } else {
                    rng.random_range(0..current_enemy_count)
                };
                rng_idx += 1;

                let target = &mut enemy.entities[target_idx];
                let target_type = target.entity_type;
                // Track damage before/after for metrics
                let prev_shield = target.current_shield.max(0.0);
                let prev_hull = target.current_hull.max(0.0);

                // Apply damage (using local variables to avoid borrow issues)
                apply_damage_fast(weapon_power, target, rng);

                // increment shots and accumulate damage stats
                stats.shots += 1;
                let new_shield = target.current_shield.max(0.0);
                let new_hull = target.current_hull.max(0.0);
                let shield_delta = (prev_shield - new_shield).max(0.0);
                let hull_delta = (prev_hull - new_hull).max(0.0);
                stats.shield_damage += shield_delta as u64;
                stats.hull_damage += hull_delta as u64;

                // Check rapid fire
                if use_rapid_fire {
                    if let Some(rf_map) = self.rapid_fire_map.get(&attacker_type) {
                        if let Some(&rf_value) = rf_map.get(&target_type) {
                            // Calculate rapid fire probability
                            // Chance to shoot again = 1 - (1 / rapid_fire_value)
                            // e.g., rapid fire 5 = 80% chance
                            let continue_probability = 1.0 - (1.0 / f32::from(rf_value));

                            // Use pre-generated random number
                            let rf_check = if rng_idx < random_rf_checks.len() {
                                random_rf_checks[rng_idx]
                            } else {
                                rng.random::<f32>()
                            };
                            rng_idx += 1;

                            if rf_check > continue_probability {
                                // Failed rapid fire check, stop shooting
                                break;
                            }
                            // Continue to shoot again
                            continue;
                        }
                    }
                }

                // No rapid fire or rapid fire not applicable
                break;
            }
        }
    }

    /// Wipe the side out without a shot being fired.
    ///
    /// The one thing v13's instant calculation has to do to the battle, and the
    /// only reason it is this small: emptying the losing party leaves the round
    /// loop's own condition false, so the loop does not run, `rounds` stays 0
    /// and the result — outcome, losses, remaining, per-slot breakdown, and
    /// everything the simulator derives from them — is assembled by exactly the
    /// code that assembles a fought battle's. A short-circuit that built its
    /// own result would be a second place for the shape of a result to be
    /// decided, and the two would drift.
    pub fn annihilate(&mut self) {
        self.entities.clear();
    }

    pub fn regenerate_shields(&mut self) {
        for entity in &mut self.entities {
            entity.regenerate_shield();
        }
    }

    #[inline]
    pub fn remove_destroyed(&mut self) {
        // Compact array: move all alive entities to the front
        // This maintains O(1) target selection by keeping alive entities at indices 0..remaining
        self.entities.retain(|e| e.is_alive);
    }

    pub fn get_losses(&self, original: &FleetComposition) -> FleetComposition {
        let mut remaining = HashMap::new();

        // Count remaining entities
        for entity in &self.entities {
            if entity.is_alive {
                *remaining.entry(entity.entity_type).or_insert(0) += 1;
            }
        }

        // Calculate losses
        let mut losses = HashMap::new();
        for (&entity_type, &original_count) in original {
            let remaining_count = remaining.get(&entity_type).copied().unwrap_or(0);
            let lost = original_count - remaining_count;
            if lost > 0 {
                losses.insert(entity_type, lost);
            }
        }

        losses
    }

    pub fn get_remaining(&self) -> FleetComposition {
        let mut remaining = HashMap::new();

        for entity in &self.entities {
            if entity.is_alive {
                *remaining.entry(entity.entity_type).or_insert(0) += 1;
            }
        }

        remaining
    }

    /// Remaining ships grouped by slot id
    pub fn get_remaining_by_slot(&self) -> HashMap<u8, FleetComposition> {
        let mut by_slot: HashMap<u8, FleetComposition> = HashMap::new();
        for entity in &self.entities {
            if entity.is_alive {
                let slot = entity.slot_id;
                let entry = by_slot.entry(slot).or_default();
                *entry.entry(entity.entity_type).or_insert(0) += 1;
            }
        }
        by_slot
    }
}

/// Apply damage from attacker to target (optimized version)
#[inline]
fn apply_damage_fast(weapon_power: u32, target: &mut Entity, rng: &mut impl Rng) {
    if !target.is_alive {
        return;
    }

    // OGame damage is exact (no randomization)
    let mut attack_power = weapon_power as f32;

    // Handle shield damage
    if attack_power < target.max_shield && target.current_shield >= 0.0 {
        // Attack is weaker than full shield - calculate percentage damage
        let damage_percentage = (attack_power / target.max_shield * 100.0).floor();

        if damage_percentage >= 1.0 {
            // Deal percentage-based damage
            let shield_damage = (damage_percentage / 100.0) * target.max_shield;
            target.current_shield -= shield_damage;

            // Handle edge case: damage percentage has decimal part
            if target.current_shield == 0.0 && damage_percentage > damage_percentage.floor() {
                target.current_shield -=
                    ((damage_percentage - damage_percentage.floor()) / 100.0) * target.max_shield;
            }
        }
        // else: Shot bounces (damage < 1% of shield)

        attack_power = 0.0; // Attack absorbed by shield
    } else if target.current_shield > 0.0 {
        // Attack is stronger than shield - break through
        attack_power -= target.current_shield;
        target.current_shield = -1.0; // Mark shield as destroyed
    }

    // Handle hull damage
    if attack_power > 0.0 {
        target.current_hull -= attack_power;

        if target.current_hull <= 0.0 {
            // Entity destroyed
            target.destroy();
        } else {
            // Check for explosion probability
            target.check_explosion(rng);
        }
    }
}

/// Main combat simulation
pub struct Combat {
    /// Borrowed from the process-wide table rather than owned — `Combat` only
    /// ever reads it, and a `Simulator` is cloned into every rayon worker.
    entity_db: &'static HashMap<EntityType, EntityStats>,
}

impl Combat {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entity_db: combat_types::entities::entity_stats(),
        }
    }

    pub fn simulate_single(
        &self,
        attacker_data: &PartyData,
        defender_data: &PartyData,
        use_rapid_fire: bool,
        collect_compositions: bool,
        rng: &mut impl Rng,
    ) -> SingleCombatResult {
        self.resolve(
            attacker_data,
            defender_data,
            use_rapid_fire,
            collect_compositions,
            InstantCalculation::Applied,
            rng,
        )
    }

    /// The same battle with v13's instant calculation left off, fought round by
    /// round however lopsided it is.
    ///
    /// The short-circuit is only allowed to be an optimisation, and a claim
    /// like that is worth exactly what tests it: `instant_calculation.rs` runs
    /// a battle down both paths and compares the two results field for field.
    /// Nothing in the engine calls this — [`Combat::simulate_single`] is the
    /// entry point, and `Simulator` goes through that — but the equivalence
    /// cannot be asserted from outside the crate without it.
    pub fn simulate_single_through_the_rounds(
        &self,
        attacker_data: &PartyData,
        defender_data: &PartyData,
        use_rapid_fire: bool,
        collect_compositions: bool,
        rng: &mut impl Rng,
    ) -> SingleCombatResult {
        self.resolve(
            attacker_data,
            defender_data,
            use_rapid_fire,
            collect_compositions,
            InstantCalculation::Skipped,
            rng,
        )
    }

    fn resolve(
        &self,
        attacker_data: &PartyData,
        defender_data: &PartyData,
        use_rapid_fire: bool,
        collect_compositions: bool,
        instant: InstantCalculation,
        rng: &mut impl Rng,
    ) -> SingleCombatResult {
        // Precompute stats
        let attacker_stats = StatsCache::new(self.entity_db, attacker_data);
        let defender_stats = StatsCache::new(self.entity_db, defender_data);

        // Create parties
        let mut attackers = Party::new(attacker_data, self.entity_db, &attacker_stats);
        let mut defenders = Party::new(defender_data, self.entity_db, &defender_stats);

        // v13's instant calculation, after the parties are built and before a
        // shot is fired: the rule is about effective attack power, and the
        // units are where the effective figures already are. See
        // `crate::instant` for what it decides and, more to the point, for the
        // battles it declines to decide.
        instant.apply(&mut attackers, &mut defenders);

        let mut round = 0u8;
        let mut round_details: Vec<RoundDetails> = Vec::new();
        let mut round_compositions: Vec<RoundComposition> = Vec::new();

        // Combat rounds (max 6)
        while round < MAX_ROUNDS
            && attackers.remaining_count() > 0
            && defenders.remaining_count() > 0
        {
            round += 1;

            // Record counts at round start
            let a_start = attackers.remaining_count();
            let d_start = defenders.remaining_count();

            // Capture per-type start if requested
            let a_start_by_type = if collect_compositions {
                attackers.count_by_type_alive()
            } else {
                HashMap::new()
            };
            let d_start_by_type = if collect_compositions {
                defenders.count_by_type_alive()
            } else {
                HashMap::new()
            };

            // Attackers shoot
            let mut a_fire = FireStats::default();
            attackers.shoot_at(&mut defenders, use_rapid_fire, rng, &mut a_fire);

            // Defenders shoot
            let mut d_fire = FireStats::default();
            defenders.shoot_at(&mut attackers, use_rapid_fire, rng, &mut d_fire);

            // Regenerate shields
            attackers.regenerate_shields();
            defenders.regenerate_shields();

            // Remove destroyed entities
            attackers.remove_destroyed();
            defenders.remove_destroyed();

            // Build round details entry
            let details = RoundDetails {
                round_number: round,
                attackers_start: a_start as u32,
                defenders_start: d_start as u32,
                attackers_destroyed: (a_start.saturating_sub(attackers.remaining_count())) as u32,
                defenders_destroyed: (d_start.saturating_sub(defenders.remaining_count())) as u32,
                attackers_end: attackers.remaining_count() as u32,
                defenders_end: defenders.remaining_count() as u32,
                attacker_damage: Some(a_fire.hull_damage),
                defender_damage: Some(d_fire.hull_damage),
                attacker_shots: Some(a_fire.shots),
                defender_shots: Some(d_fire.shots),
                attacker_shield_damage: Some(a_fire.shield_damage),
                defender_shield_damage: Some(d_fire.shield_damage),
            };
            round_details.push(details);

            if collect_compositions {
                let a_end_by_type = attackers.count_by_type_alive();
                let d_end_by_type = defenders.count_by_type_alive();

                let mut a_destroyed: FleetComposition = HashMap::new();
                for (&t, &start_c) in &a_start_by_type {
                    let end_c = a_end_by_type.get(&t).copied().unwrap_or(0);
                    if start_c > end_c {
                        a_destroyed.insert(t, start_c - end_c);
                    }
                }
                let mut d_destroyed: FleetComposition = HashMap::new();
                for (&t, &start_c) in &d_start_by_type {
                    let end_c = d_end_by_type.get(&t).copied().unwrap_or(0);
                    if start_c > end_c {
                        d_destroyed.insert(t, start_c - end_c);
                    }
                }

                round_compositions.push(RoundComposition {
                    round_number: round,
                    attacker_by_type_start: a_start_by_type.clone(),
                    defender_by_type_start: d_start_by_type.clone(),
                    attacker_by_type_destroyed: a_destroyed,
                    defender_by_type_destroyed: d_destroyed,
                    attacker_by_type_end: a_end_by_type,
                    defender_by_type_end: d_end_by_type,
                });
            }
        }

        // Determine outcome
        let attacker_alive = attackers.remaining_count() > 0;
        let defender_alive = defenders.remaining_count() > 0;

        let outcome = match (attacker_alive, defender_alive) {
            (true, false) => CombatOutcome::AttackersWin,
            (false, true) => CombatOutcome::DefendersWin,
            _ => CombatOutcome::Draw,
        };

        SingleCombatResult {
            outcome,
            rounds: round,
            attacker_losses: attackers.get_losses(&attacker_data.entities),
            defender_losses: defenders.get_losses(&defender_data.entities),
            attacker_remaining: attackers.get_remaining(),
            defender_remaining: defenders.get_remaining(),
            round_details: Some(round_details),
            round_compositions: if collect_compositions {
                Some(round_compositions)
            } else {
                None
            },
            attacker_slots: None,
            defender_slots: None,
        }
    }

    /// Simulate combat with explicit slots (A1/A2, D1/D2), returning per-slot results
    // Long, and legitimately flagged: this is the round loop with slot
    // bookkeeping threaded through it. Splitting it is real work with real
    // risk to combat accuracy, so it is allowed here rather than done badly
    // as a side effect of adopting a linter.
    #[allow(clippy::too_many_lines)]
    pub fn simulate_single_with_slots(
        &self,
        attacker_slots: &[(String, PartyData)],
        defender_slots: &[(String, PartyData)],
        use_rapid_fire: bool,
        collect_compositions: bool,
        rng: &mut impl Rng,
    ) -> SingleCombatResult {
        // Precompute per-slot stats and build combined parties tagging slot ids
        let mut attackers = Party {
            entities: Vec::new(),
            rapid_fire_map: HashMap::new(),
        };
        let mut defenders = Party {
            entities: Vec::new(),
            rapid_fire_map: HashMap::new(),
        };

        let mut attacker_original_per_slot: HashMap<u8, FleetComposition> = HashMap::new();
        let mut defender_original_per_slot: HashMap<u8, FleetComposition> = HashMap::new();

        // Helper to extend party from a slot
        let extend_party = |party: &mut Party, slot_index: usize, data: &PartyData| {
            let stats_cache = StatsCache::new(self.entity_db, data);
            let slot_id = (slot_index + 1) as u8;
            let mut original: FleetComposition = HashMap::new();
            for (&entity_type, &count) in &data.entities {
                if count == 0 {
                    continue;
                }
                if let Some(base_stats) = self.entity_db.get(&entity_type) {
                    if let Some(modified_stats) = stats_cache.get(entity_type) {
                        // Store RF info
                        if !base_stats.rapid_fire_against.is_empty() {
                            party
                                .rapid_fire_map
                                .insert(entity_type, base_stats.rapid_fire_against.clone());
                        }
                        for _ in 0..count {
                            party.entities.push(Entity::new_with_slot(
                                entity_type,
                                modified_stats.weapon,
                                modified_stats.shield,
                                modified_stats.hull,
                                slot_id,
                            ));
                        }
                        *original.entry(entity_type).or_insert(0) += count;
                    }
                }
            }
            original
        };

        for (idx, (_slot_name, data)) in attacker_slots.iter().enumerate() {
            let original = extend_party(&mut attackers, idx, data);
            attacker_original_per_slot.insert((idx + 1) as u8, original);
        }
        for (idx, (_slot_name, data)) in defender_slots.iter().enumerate() {
            let original = extend_party(&mut defenders, idx, data);
            defender_original_per_slot.insert((idx + 1) as u8, original);
        }

        // v13's instant calculation applies here too, and on the same figures.
        // Combined attack power is a property of a *side*, and slots are only
        // how one side's fleet is reported afterwards — an attacker who splits
        // a fleet across A1 and A2 has not changed what it is worth, so a rule
        // that read the slots separately would answer differently for the same
        // ships. The parties below are already the whole side merged, which is
        // exactly what the rule wants to see.
        InstantCalculation::Applied.apply(&mut attackers, &mut defenders);

        // Run the same round loop with metrics
        let mut round = 0u8;
        let mut round_details: Vec<RoundDetails> = Vec::new();
        let mut round_compositions: Vec<RoundComposition> = Vec::new();
        while round < MAX_ROUNDS
            && attackers.remaining_count() > 0
            && defenders.remaining_count() > 0
        {
            round += 1;
            let a_start = attackers.remaining_count();
            let d_start = defenders.remaining_count();

            let a_start_by_type = if collect_compositions {
                attackers.count_by_type_alive()
            } else {
                HashMap::new()
            };
            let d_start_by_type = if collect_compositions {
                defenders.count_by_type_alive()
            } else {
                HashMap::new()
            };

            let mut a_fire = FireStats::default();
            attackers.shoot_at(&mut defenders, use_rapid_fire, rng, &mut a_fire);

            let mut d_fire = FireStats::default();
            defenders.shoot_at(&mut attackers, use_rapid_fire, rng, &mut d_fire);

            attackers.regenerate_shields();
            defenders.regenerate_shields();
            attackers.remove_destroyed();
            defenders.remove_destroyed();

            let details = RoundDetails {
                round_number: round,
                attackers_start: a_start as u32,
                defenders_start: d_start as u32,
                attackers_destroyed: (a_start.saturating_sub(attackers.remaining_count())) as u32,
                defenders_destroyed: (d_start.saturating_sub(defenders.remaining_count())) as u32,
                attackers_end: attackers.remaining_count() as u32,
                defenders_end: defenders.remaining_count() as u32,
                attacker_damage: Some(a_fire.hull_damage),
                defender_damage: Some(d_fire.hull_damage),
                attacker_shots: Some(a_fire.shots),
                defender_shots: Some(d_fire.shots),
                attacker_shield_damage: Some(a_fire.shield_damage),
                defender_shield_damage: Some(d_fire.shield_damage),
            };
            round_details.push(details);

            if collect_compositions {
                let a_end_by_type = attackers.count_by_type_alive();
                let d_end_by_type = defenders.count_by_type_alive();

                let mut a_destroyed: FleetComposition = HashMap::new();
                for (&t, &start_c) in &a_start_by_type {
                    let end_c = a_end_by_type.get(&t).copied().unwrap_or(0);
                    if start_c > end_c {
                        a_destroyed.insert(t, start_c - end_c);
                    }
                }
                let mut d_destroyed: FleetComposition = HashMap::new();
                for (&t, &start_c) in &d_start_by_type {
                    let end_c = d_end_by_type.get(&t).copied().unwrap_or(0);
                    if start_c > end_c {
                        d_destroyed.insert(t, start_c - end_c);
                    }
                }

                round_compositions.push(RoundComposition {
                    round_number: round,
                    attacker_by_type_start: a_start_by_type.clone(),
                    defender_by_type_start: d_start_by_type.clone(),
                    attacker_by_type_destroyed: a_destroyed,
                    defender_by_type_destroyed: d_destroyed,
                    attacker_by_type_end: a_end_by_type,
                    defender_by_type_end: d_end_by_type,
                });
            }
        }

        let outcome = match (
            attackers.remaining_count() > 0,
            defenders.remaining_count() > 0,
        ) {
            (true, false) => CombatOutcome::AttackersWin,
            (false, true) => CombatOutcome::DefendersWin,
            _ => CombatOutcome::Draw,
        };

        // Aggregate totals
        let attacker_remaining = attackers.get_remaining();
        let defender_remaining = defenders.get_remaining();

        let mut attacker_original_total: FleetComposition = HashMap::new();
        for fleet in attacker_original_per_slot.values() {
            for (&t, &c) in fleet {
                *attacker_original_total.entry(t).or_insert(0) += c;
            }
        }
        let mut defender_original_total: FleetComposition = HashMap::new();
        for fleet in defender_original_per_slot.values() {
            for (&t, &c) in fleet {
                *defender_original_total.entry(t).or_insert(0) += c;
            }
        }

        // Per-slot remaining and losses
        let a_remaining_by_slot = attackers.get_remaining_by_slot();
        let d_remaining_by_slot = defenders.get_remaining_by_slot();

        let mut a_slot_results: Vec<SlotResult> = Vec::new();
        for (slot_id, original) in &attacker_original_per_slot {
            let remaining = a_remaining_by_slot
                .get(slot_id)
                .cloned()
                .unwrap_or_default();
            // losses = original - remaining
            let mut losses: FleetComposition = HashMap::new();
            for (&t, &oc) in original {
                let rc = remaining.get(&t).copied().unwrap_or(0);
                if oc > rc {
                    losses.insert(t, oc - rc);
                }
            }
            a_slot_results.push(SlotResult {
                slot_id: format!("A{}", *slot_id as usize),
                initial: original.clone(),
                losses,
                remaining,
            });
        }
        let mut d_slot_results: Vec<SlotResult> = Vec::new();
        for (slot_id, original) in &defender_original_per_slot {
            let remaining = d_remaining_by_slot
                .get(slot_id)
                .cloned()
                .unwrap_or_default();
            let mut losses: FleetComposition = HashMap::new();
            for (&t, &oc) in original {
                let rc = remaining.get(&t).copied().unwrap_or(0);
                if oc > rc {
                    losses.insert(t, oc - rc);
                }
            }
            d_slot_results.push(SlotResult {
                slot_id: format!("D{}", *slot_id as usize),
                initial: original.clone(),
                losses,
                remaining,
            });
        }

        SingleCombatResult {
            outcome,
            rounds: round,
            attacker_losses: attackers.get_losses(&attacker_original_total),
            defender_losses: defenders.get_losses(&defender_original_total),
            attacker_remaining,
            defender_remaining,
            round_details: Some(round_details),
            round_compositions: if collect_compositions {
                Some(round_compositions)
            } else {
                None
            },
            attacker_slots: Some(a_slot_results),
            defender_slots: Some(d_slot_results),
        }
    }
}

impl Default for Combat {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatOutcome {
    AttackersWin,
    DefendersWin,
    Draw,
}

pub struct SingleCombatResult {
    pub outcome: CombatOutcome,
    pub rounds: u8,
    pub attacker_losses: FleetComposition,
    pub defender_losses: FleetComposition,
    pub attacker_remaining: FleetComposition,
    pub defender_remaining: FleetComposition,
    pub round_details: Option<Vec<RoundDetails>>,
    pub round_compositions: Option<Vec<RoundComposition>>,
    pub attacker_slots: Option<Vec<SlotResult>>,
    pub defender_slots: Option<Vec<SlotResult>>,
}
