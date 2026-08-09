use combat_types::entities::load_entity_stats;
/// Build comprehensive combat reports from simulation results
use combat_types::{
    CombatOutcome, CombatReport, CombatRequest, CombatResults, DebrisField, EconomicSummary,
    EntityStats, EntityType, FleetComposition, FleetSnapshot, HarvestInfo, Participant,
    PlanetResources, ResourceCost, SimulationResult, classify_battle_type,
};
use std::collections::HashMap;

/// Builder for creating detailed combat reports
pub struct ReportBuilder {
    entity_db: HashMap<EntityType, EntityStats>,
}

impl ReportBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entity_db: load_entity_stats(),
        }
    }

    /// Convert a simulation result into a comprehensive combat report
    #[must_use]
    pub fn build_report(
        &self,
        request: &CombatRequest,
        result: &SimulationResult,
        battle_id: Option<String>,
        timestamp: Option<u64>,
    ) -> CombatReport {
        // Generate battle ID if not provided
        let battle_id = battle_id.unwrap_or_else(|| {
            let ts = timestamp.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            });
            CombatReport::generate_battle_id(ts, None, None)
        });

        let timestamp = timestamp.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        // Create participant info (using placeholder data - you can customize)
        let attacker = Participant {
            name: "Attacker".to_string(),
            player_id: None,
            coordinates: None,
            technology: request.attacker.technology,
            alliance: None,
        };

        let defender = Participant {
            name: "Defender".to_string(),
            player_id: None,
            coordinates: None,
            technology: request.defender.technology,
            alliance: None,
        };

        // Build fleet snapshots
        let attacker_fleet_start = self.build_fleet_snapshot(&request.attacker.entities);
        let defender_fleet_start = self.build_fleet_snapshot(&request.defender.entities);
        let attacker_losses = self.build_fleet_snapshot(&result.attacker_losses);
        let defender_losses = self.build_fleet_snapshot(&result.defender_losses);
        let attacker_fleet_end = self.build_fleet_snapshot(&result.attacker_remaining);
        let defender_fleet_end = self.build_fleet_snapshot(&result.defender_remaining);

        // Build economic summary
        let moon_chance = CombatReport::calculate_moon_chance(&result.debris_field);
        let recyclers_needed = CombatReport::calculate_recyclers_needed(&result.debris_field);
        let harvest_time =
            CombatReport::estimate_harvest_time(recyclers_needed, &result.debris_field);

        let economics = EconomicSummary {
            debris_field: result.debris_field.clone(),
            moon_chance,
            plunder: result.loot.clone(),
            attacker_losses_cost: attacker_losses.total_value.clone(),
            defender_losses_cost: defender_losses.total_value.clone(),
            attacker_profit: result.attacker_profit,
            defender_profit: result.defender_profit,
            harvest_info: if result.debris_field.total() > 0 {
                Some(HarvestInfo {
                    recyclers_needed,
                    harvest_time_seconds: harvest_time,
                })
            } else {
                None
            },
        };

        // Classify battle type
        let battle_type =
            classify_battle_type(&request.attacker.entities, &request.defender.entities);

        CombatReport {
            battle_id,
            timestamp,
            battle_type,
            attacker,
            defender,
            outcome: result.outcome.clone(),
            rounds: result.rounds,
            attacker_fleet_start,
            defender_fleet_start,
            attacker_losses,
            defender_losses,
            attacker_fleet_end,
            defender_fleet_end,
            economics,
            round_details: None, // Can be populated if you track per-round data
            moon_destruction: None, // Can be populated for Death Star attacks
            simulation_count: 1, // Single result
            duration_ms: 0,      // Will be set by caller
        }
    }

    /// Build multiple reports from combat results
    #[must_use]
    pub fn build_reports(
        &self,
        request: &CombatRequest,
        results: &CombatResults,
    ) -> Vec<CombatReport> {
        let base_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        results
            .results
            .iter()
            .enumerate()
            .map(|(idx, result)| {
                let timestamp = base_timestamp + idx as u64;
                let battle_id = CombatReport::generate_battle_id(timestamp, None, None);

                let mut report =
                    self.build_report(request, result, Some(battle_id), Some(timestamp));
                report.simulation_count = results.simulations;
                report.duration_ms = results.duration_ms;
                report
            })
            .collect()
    }

    /// Build an aggregated summary report from multiple simulations
    #[must_use]
    pub fn build_summary_report(
        &self,
        request: &CombatRequest,
        results: &CombatResults,
    ) -> CombatReport {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let battle_id = format!("cr-summary-{timestamp}");

        // Calculate average results
        let avg_result = Self::calculate_average_result(results);

        let mut report = self.build_report(request, &avg_result, Some(battle_id), Some(timestamp));
        report.simulation_count = results.simulations;
        report.duration_ms = results.duration_ms;

        report
    }

    /// Calculate average/most common result from multiple simulations
    fn calculate_average_result(results: &CombatResults) -> SimulationResult {
        assert!(!results.results.is_empty(), "No results to average");

        // Calculate average losses and remaining
        let avg_attacker_losses = Self::average_fleet(
            &results
                .results
                .iter()
                .map(|r| &r.attacker_losses)
                .collect::<Vec<_>>(),
        );
        let avg_defender_losses = Self::average_fleet(
            &results
                .results
                .iter()
                .map(|r| &r.defender_losses)
                .collect::<Vec<_>>(),
        );
        let avg_attacker_remaining = Self::average_fleet(
            &results
                .results
                .iter()
                .map(|r| &r.attacker_remaining)
                .collect::<Vec<_>>(),
        );
        let avg_defender_remaining = Self::average_fleet(
            &results
                .results
                .iter()
                .map(|r| &r.defender_remaining)
                .collect::<Vec<_>>(),
        );

        // Determine outcome based on averaged remaining fleets (more accurate than "most common")
        let attacker_has_ships = avg_attacker_remaining.values().any(|&count| count > 0);
        let defender_has_ships = avg_defender_remaining.values().any(|&count| count > 0);

        let outcome = match (attacker_has_ships, defender_has_ships) {
            (true, false) => CombatOutcome::AttackersWin,
            (false, true) => CombatOutcome::DefendersWin,
            // Both sides standing, or neither: the engine has no third word for
            // it. Spelled as an or-pattern rather than a wildcard so adding a
            // fourth outcome still has to visit this arm.
            (true, true) | (false, false) => CombatOutcome::Draw,
        };

        // Average debris
        let avg_debris_metal = results
            .results
            .iter()
            .map(|r| r.debris_field.metal)
            .sum::<u64>()
            / u64::from(results.simulations);
        let avg_debris_crystal = results
            .results
            .iter()
            .map(|r| r.debris_field.crystal)
            .sum::<u64>()
            / u64::from(results.simulations);

        // Average loot
        let avg_loot_metal = results.results.iter().map(|r| r.loot.metal).sum::<u64>()
            / u64::from(results.simulations);
        let avg_loot_crystal = results.results.iter().map(|r| r.loot.crystal).sum::<u64>()
            / u64::from(results.simulations);
        let avg_loot_deut = results
            .results
            .iter()
            .map(|r| r.loot.deuterium)
            .sum::<u64>()
            / u64::from(results.simulations);

        // Average profit
        let avg_attacker_profit = results
            .results
            .iter()
            .map(|r| r.attacker_profit)
            .sum::<i64>()
            / i64::from(results.simulations);
        let avg_defender_profit = results
            .results
            .iter()
            .map(|r| r.defender_profit)
            .sum::<i64>()
            / i64::from(results.simulations);

        SimulationResult {
            outcome,
            rounds: (results.average_rounds.round() as u8),
            attacker_losses: avg_attacker_losses,
            defender_losses: avg_defender_losses,
            attacker_remaining: avg_attacker_remaining,
            defender_remaining: avg_defender_remaining,
            debris_field: DebrisField {
                metal: avg_debris_metal,
                crystal: avg_debris_crystal,
            },
            loot: PlanetResources {
                metal: avg_loot_metal,
                crystal: avg_loot_crystal,
                deuterium: avg_loot_deut,
            },
            attacker_profit: avg_attacker_profit,
            defender_profit: avg_defender_profit,
            round_details: None,
            round_compositions: None,
            round_compositions_by_slot: None,
            attacker_slots: None,
            defender_slots: None,
        }
    }

    /// Average fleet compositions
    fn average_fleet(fleets: &[&FleetComposition]) -> FleetComposition {
        if fleets.is_empty() {
            return HashMap::new();
        }

        let mut result = HashMap::new();
        let count = fleets.len() as u32;

        // Get all entity types
        let mut all_types = std::collections::HashSet::new();
        for fleet in fleets {
            for &entity_type in fleet.keys() {
                all_types.insert(entity_type);
            }
        }

        // Average each type
        for entity_type in all_types {
            let sum: u32 = fleets
                .iter()
                .map(|f| f.get(&entity_type).copied().unwrap_or(0))
                .sum();
            let avg = sum / count;
            if avg > 0 {
                result.insert(entity_type, avg);
            }
        }

        result
    }

    /// Build fleet snapshot with cost calculation
    fn build_fleet_snapshot(&self, fleet: &FleetComposition) -> FleetSnapshot {
        let mut total_value = ResourceCost::default();

        for (&entity_type, &count) in fleet {
            if let Some(stats) = self.entity_db.get(&entity_type) {
                total_value.metal += u64::from(stats.cost_metal) * u64::from(count);
                total_value.crystal += u64::from(stats.cost_crystal) * u64::from(count);
                total_value.deuterium += u64::from(stats.cost_deuterium) * u64::from(count);
            }
        }

        FleetSnapshot {
            ships: fleet.clone(),
            total_value,
        }
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}
