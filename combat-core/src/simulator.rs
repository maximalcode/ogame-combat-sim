use crate::combat::{Combat, CombatOutcome};
use crate::economics;
use crate::scaling::{
    calculate_downscale_factor, downscale_party, upscale_result_with_originals,
    upscale_round_compositions, upscale_round_compositions_by_slot, upscale_round_details,
    upscale_slot_results,
};
use combat_types::entities::entity_stats;
use combat_types::{CombatRequest, CombatResults, PartyData, PlanetResources, SimulationResult};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rayon::prelude::*;
use std::sync::Arc;

pub struct Simulator {
    combat: Arc<Combat>,
}

impl Simulator {
    #[must_use]
    pub fn new() -> Self {
        // Configure Rayon thread pool for optimal performance
        // Use 75% of available CPUs to leave room for other processes
        let total_cpus = num_cpus::get();
        let sim_threads = ((total_cpus * 3) / 4).max(1);

        rayon::ThreadPoolBuilder::new()
            .num_threads(sim_threads)
            .thread_name(|i| format!("combat-sim-{i}"))
            .build_global()
            .ok(); // Ignore error if already initialized

        Self {
            combat: Arc::new(Combat::new()),
        }
    }

    /// Run a single simulation with slots (A1/A2, D1/D2) and return per-slot enriched result
    fn simulate_once_with_slots(
        &self,
        attacker_slots: &[(String, PartyData)],
        defender_slots: &[(String, PartyData)],
        use_rapid_fire: bool,
        planet_resources: Option<&PlanetResources>,
        debris_percentage: f32,
        collect_compositions: bool,
    ) -> SimulationResult {
        let mut rng = SmallRng::from_os_rng();
        let single = self.combat.simulate_single_with_slots(
            attacker_slots,
            defender_slots,
            use_rapid_fire,
            collect_compositions,
            &mut rng,
        );

        // Derive aggregated economic data
        let entity_db = entity_stats();
        let debris_field = economics::calculate_debris(
            &single.attacker_losses,
            &single.defender_losses,
            entity_db,
            debris_percentage,
        );

        let loot = if let Some(resources) = planet_resources {
            let cargo_capacity =
                economics::calculate_cargo_capacity(&single.attacker_remaining, entity_db);
            economics::calculate_loot(resources, cargo_capacity)
        } else {
            PlanetResources::default()
        };

        let attacker_profit = economics::calculate_attacker_profit(
            &debris_field,
            &loot,
            &single.attacker_losses,
            entity_db,
        );
        let defender_profit =
            economics::calculate_defender_profit(&debris_field, &single.defender_losses, entity_db);

        SimulationResult {
            outcome: match single.outcome {
                CombatOutcome::AttackersWin => combat_types::CombatOutcome::AttackersWin,
                CombatOutcome::DefendersWin => combat_types::CombatOutcome::DefendersWin,
                CombatOutcome::Draw => combat_types::CombatOutcome::Draw,
            },
            rounds: single.rounds,
            attacker_losses: single.attacker_losses,
            defender_losses: single.defender_losses,
            attacker_remaining: single.attacker_remaining,
            defender_remaining: single.defender_remaining,
            debris_field,
            loot,
            attacker_profit,
            defender_profit,
            round_details: single.round_details,
            round_compositions: single.round_compositions,
            round_compositions_by_slot: None,
            attacker_slots: single.attacker_slots,
            defender_slots: single.defender_slots,
        }
    }

    /// Run a single simulation (internal helper)
    fn simulate_once_internal(
        &self,
        attacker: &PartyData,
        defender: &PartyData,
        use_rapid_fire: bool,
        planet_resources: Option<&PlanetResources>,
        debris_percentage: f32,
        collect_compositions: bool,
    ) -> SimulationResult {
        // Use SmallRng for 3x faster RNG (non-cryptographic but sufficient for simulations)
        let mut rng = SmallRng::from_os_rng();
        let result = self.combat.simulate_single(
            attacker,
            defender,
            use_rapid_fire,
            collect_compositions,
            &mut rng,
        );

        // Load entity database for economic calculations
        let entity_db = entity_stats();

        // Calculate debris field
        let debris_field = economics::calculate_debris(
            &result.attacker_losses,
            &result.defender_losses,
            entity_db,
            debris_percentage,
        );

        // Calculate loot if planet resources are provided
        let loot = if let Some(resources) = planet_resources {
            let cargo_capacity =
                economics::calculate_cargo_capacity(&result.attacker_remaining, entity_db);
            economics::calculate_loot(resources, cargo_capacity)
        } else {
            PlanetResources::default()
        };

        // Calculate profits
        let attacker_profit = economics::calculate_attacker_profit(
            &debris_field,
            &loot,
            &result.attacker_losses,
            entity_db,
        );

        let defender_profit =
            economics::calculate_defender_profit(&debris_field, &result.defender_losses, entity_db);

        SimulationResult {
            outcome: match result.outcome {
                CombatOutcome::AttackersWin => combat_types::CombatOutcome::AttackersWin,
                CombatOutcome::DefendersWin => combat_types::CombatOutcome::DefendersWin,
                CombatOutcome::Draw => combat_types::CombatOutcome::Draw,
            },
            rounds: result.rounds,
            attacker_losses: result.attacker_losses,
            defender_losses: result.defender_losses,
            attacker_remaining: result.attacker_remaining,
            defender_remaining: result.defender_remaining,
            debris_field,
            loot,
            attacker_profit,
            defender_profit,
            round_details: result.round_details,
            round_compositions: result.round_compositions,
            round_compositions_by_slot: None,
            attacker_slots: None,
            defender_slots: None,
        }
    }

    /// Run multiple simulations in parallel and aggregate results
    // Long for the same reason as `simulate_single_with_slots`: downscaling,
    // slots and upscaling all have to agree, and the agreement is easier to
    // read in one place than spread over helpers that must be called in order.
    #[allow(clippy::too_many_lines)]
    pub fn simulate_multiple(&self, request: &CombatRequest) -> CombatResults {
        let start = std::time::Instant::now();

        // Player and alliance classes are effective technology levels, so each
        // side's bonuses are folded into its technology once, here, before any
        // of it reaches combat. Everything downstream — downscaling, the round
        // loop, the stat cache — goes on seeing a plain `PartyData` and never
        // learns that classes exist. Per side, because the request carries one
        // bonus block per side and they are free to differ.
        let attacker = request.effective_attacker();
        let defender = request.effective_defender();

        // Check if we should downscale for large battles
        let downscale_factor = match request.enable_downscaling {
            Some(false) => 1, // Force disable downscaling
            // Explicit opt-in and the default behave identically: the factor
            // calculation already returns 1 when the fleets are small enough.
            Some(true) | None => calculate_downscale_factor(&attacker, &defender),
        };

        let (attacker_data, defender_data) = if downscale_factor > 1
            && request.attacker_slots.is_none()
            && request.defender_slots.is_none()
        {
            // Downscale fleets for massive performance gain
            (
                downscale_party(&attacker, downscale_factor),
                downscale_party(&defender, downscale_factor),
            )
        } else {
            // Moved, not cloned: these are already owned copies at effective
            // levels, and on the path this branch serves — no downscaling, so
            // fleets of any size up to ten million ships — the clone was of the
            // whole composition for nothing.
            (attacker, defender)
        };
        let used_downscaling_non_slot = downscale_factor > 1
            && request.attacker_slots.is_none()
            && request.defender_slots.is_none();

        // For weak computers: limit parallelism to avoid overwhelming the system
        // Use chunk-based parallelism for better cache locality
        let chunk_size = if request.simulations < 10 {
            1
        } else {
            (request.simulations / num_cpus::get() as u32).max(1)
        };

        // Store original fleets for precision-preserving upscaling
        let original_attacker = request.attacker.entities.clone();
        let original_defender = request.defender.entities.clone();

        // Prepare slot-mode structures (original and optionally downscaled)
        let (
            a_slots_orig,
            d_slots_orig,
            a_slots_scaled,
            d_slots_scaled,
            a_slot_orig_map,
            d_slot_orig_map,
        ) = if let (Some(a_slots), Some(d_slots)) =
            (&request.attacker_slots, &request.defender_slots)
        {
            // Build original vectors for simulation. A side's bonuses belong to
            // the player, so every slot on that side fights under them.
            let a_orig: Vec<(String, combat_types::PartyData)> = a_slots
                .iter()
                .map(|s| {
                    (
                        s.id.clone(),
                        s.data
                            .at_effective_levels(request.attacker_bonuses.as_ref()),
                    )
                })
                .collect();
            let d_orig: Vec<(String, combat_types::PartyData)> = d_slots
                .iter()
                .map(|s| {
                    (
                        s.id.clone(),
                        s.data
                            .at_effective_levels(request.defender_bonuses.as_ref()),
                    )
                })
                .collect();

            // Original per-slot totals for precision restoration
            let mut a_map: std::collections::HashMap<String, combat_types::FleetComposition> =
                std::collections::HashMap::new();
            for s in a_slots {
                a_map.insert(s.id.clone(), s.data.entities.clone());
            }
            let mut d_map: std::collections::HashMap<String, combat_types::FleetComposition> =
                std::collections::HashMap::new();
            for s in d_slots {
                d_map.insert(s.id.clone(), s.data.entities.clone());
            }

            // Downscaled vectors if needed, taken from the slots above so the
            // scaled battle is fought at the same effective levels as the
            // unscaled one.
            let (a_scaled, d_scaled) = if downscale_factor > 1 {
                let a_s: Vec<(String, combat_types::PartyData)> = a_orig
                    .iter()
                    .map(|(id, data)| {
                        (
                            id.clone(),
                            crate::scaling::downscale_party(data, downscale_factor),
                        )
                    })
                    .collect();
                let d_s: Vec<(String, combat_types::PartyData)> = d_orig
                    .iter()
                    .map(|(id, data)| {
                        (
                            id.clone(),
                            crate::scaling::downscale_party(data, downscale_factor),
                        )
                    })
                    .collect();
                (Some(a_s), Some(d_s))
            } else {
                (None, None)
            };

            (
                Some(a_orig),
                Some(d_orig),
                a_scaled,
                d_scaled,
                Some(std::sync::Arc::new(a_map)),
                Some(std::sync::Arc::new(d_map)),
            )
        } else {
            (None, None, None, None, None, None)
        };

        let a_slots_orig = a_slots_orig.map(std::sync::Arc::new);
        let d_slots_orig = d_slots_orig.map(std::sync::Arc::new);
        let a_slots_scaled = a_slots_scaled.map(std::sync::Arc::new);
        let d_slots_scaled = d_slots_scaled.map(std::sync::Arc::new);

        let used_downscaling_slots = downscale_factor > 1
            && request.attacker_slots.is_some()
            && request.defender_slots.is_some();

        // Run simulations in parallel with controlled chunk size
        let simulation_results: Vec<SimulationResult> = (0..request.simulations)
            .into_par_iter()
            .with_max_len(chunk_size as usize)
            .map(|_| {
                let collect = request.enable_round_compositions.unwrap_or(false);
                let result = if let (Some(a_o_arc), Some(d_o_arc)) = (&a_slots_orig, &d_slots_orig)
                {
                    // Choose scaled or original slots based on factor
                    if used_downscaling_slots {
                        let a_s_arc = a_slots_scaled.as_ref().unwrap();
                        let d_s_arc = d_slots_scaled.as_ref().unwrap();
                        self.simulate_once_with_slots(
                            a_s_arc.as_slice(),
                            d_s_arc.as_slice(),
                            request.use_rapid_fire,
                            request.planet_resources.as_ref(),
                            request.debris_percentage,
                            collect,
                        )
                    } else {
                        self.simulate_once_with_slots(
                            a_o_arc.as_slice(),
                            d_o_arc.as_slice(),
                            request.use_rapid_fire,
                            request.planet_resources.as_ref(),
                            request.debris_percentage,
                            collect,
                        )
                    }
                } else {
                    self.simulate_once_internal(
                        &attacker_data,
                        &defender_data,
                        request.use_rapid_fire,
                        request.planet_resources.as_ref(),
                        request.debris_percentage,
                        collect,
                    )
                };

                // Scale results back up if we downscaled (with precision preservation)
                if used_downscaling_non_slot || used_downscaling_slots {
                    let mut up = upscale_result_with_originals(
                        &result,
                        downscale_factor,
                        &original_attacker,
                        &original_defender,
                    );
                    // Upscale round compositions for user-facing numbers
                    up.round_compositions = upscale_round_compositions(
                        up.round_compositions.as_deref(),
                        downscale_factor,
                    );
                    up.round_compositions_by_slot = upscale_round_compositions_by_slot(
                        up.round_compositions_by_slot.as_ref(),
                        downscale_factor,
                    );
                    // Upscale RoundDetails (shots, hull damage, shield absorb, totals)
                    up.round_details =
                        upscale_round_details(up.round_details.as_deref(), downscale_factor);
                    // Upscale per-slot results if present
                    if used_downscaling_slots {
                        if let Some(a_slots) = &up.attacker_slots {
                            if let Some(a_map) = &a_slot_orig_map {
                                up.attacker_slots = Some(upscale_slot_results(
                                    a_slots,
                                    downscale_factor,
                                    a_map,
                                    'A',
                                ));
                            }
                        }
                        if let Some(d_slots) = &up.defender_slots {
                            if let Some(d_map) = &d_slot_orig_map {
                                up.defender_slots = Some(upscale_slot_results(
                                    d_slots,
                                    downscale_factor,
                                    d_map,
                                    'D',
                                ));
                            }
                        }
                    }
                    up
                } else {
                    result
                }
            })
            .collect();

        // Aggregate results
        let mut results = CombatResults::new(request.simulations);
        let mut total_rounds = 0u64;

        for result in simulation_results {
            total_rounds += u64::from(result.rounds);
            results.add_result(result);
        }

        results.duration_ms = start.elapsed().as_millis() as u64;
        results.average_rounds = total_rounds as f64 / f64::from(request.simulations);

        results
    }
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use combat_types::{FleetComposition, Technology};
    use std::collections::HashMap;

    fn create_test_request(
        attacker_fleet: FleetComposition,
        defender_fleet: FleetComposition,
        simulations: u32,
    ) -> CombatRequest {
        CombatRequest {
            attacker: PartyData {
                technology: Technology {
                    weapon: 0,
                    shield: 0,
                    armour: 0,
                    ..Default::default()
                },
                entities: attacker_fleet,
                ..Default::default()
            },
            defender: PartyData {
                technology: Technology {
                    weapon: 0,
                    shield: 0,
                    armour: 0,
                    ..Default::default()
                },
                entities: defender_fleet,
                ..Default::default()
            },
            attacker_slots: None,
            defender_slots: None,
            planet_resources: None,
            debris_percentage: 30.0,
            use_rapid_fire: true,
            simulations,
            enable_downscaling: None,
            enable_round_compositions: None,
            universe_settings: None,
            attacker_bonuses: None,
            defender_bonuses: None,
            plunder_percentage: 50,
        }
    }

    #[test]
    fn test_simple_combat() {
        let simulator = Simulator::new();

        let mut attacker_fleet = HashMap::new();
        attacker_fleet.insert(204, 10); // 10 Light Fighters

        let mut defender_fleet = HashMap::new();
        defender_fleet.insert(401, 5); // 5 Rocket Launchers

        let request = create_test_request(attacker_fleet, defender_fleet, 1);

        let results = simulator.simulate_multiple(&request);

        // Basic sanity checks
        assert!(results.results[0].rounds > 0 && results.results[0].rounds <= 6);

        // Either side should have losses
        let total_losses = results.results[0].attacker_losses.values().sum::<u32>()
            + results.results[0].defender_losses.values().sum::<u32>();
        assert!(total_losses > 0, "Combat should result in some losses");
    }

    #[test]
    fn test_multiple_simulations() {
        let simulator = Simulator::new();

        let mut attacker_fleet = HashMap::new();
        attacker_fleet.insert(204, 10);

        let mut defender_fleet = HashMap::new();
        defender_fleet.insert(401, 5);

        let request = create_test_request(attacker_fleet, defender_fleet, 100);

        let results = simulator.simulate_multiple(&request);

        assert_eq!(results.simulations, 100);
        assert_eq!(
            results.attacker_wins + results.defender_wins + results.draws,
            100
        );
        assert_eq!(results.results.len(), 100);
    }

    #[test]
    fn test_overwhelming_attacker() {
        let simulator = Simulator::new();

        let mut attacker_fleet = HashMap::new();
        attacker_fleet.insert(206, 100); // 100 Cruisers

        let mut defender_fleet = HashMap::new();
        defender_fleet.insert(401, 1); // 1 Rocket Launcher

        let request = create_test_request(attacker_fleet, defender_fleet, 10);

        let results = simulator.simulate_multiple(&request);

        // Attackers should win all battles
        assert_eq!(results.attacker_wins, 10);
        assert_eq!(results.defender_wins, 0);
    }

    #[test]
    fn test_deathstar_rapid_fire() {
        let simulator = Simulator::new();

        let mut attacker_fleet = HashMap::new();
        attacker_fleet.insert(214, 1); // 1 Deathstar

        let mut defender_fleet = HashMap::new();
        defender_fleet.insert(202, 100); // 100 Small Cargo

        let request = create_test_request(attacker_fleet, defender_fleet, 1);

        let results = simulator.simulate_multiple(&request);

        // Deathstar should win
        assert_eq!(
            results.results[0].outcome,
            combat_types::CombatOutcome::AttackersWin
        );

        // All defenders should be destroyed due to rapid fire
        assert_eq!(results.results[0].defender_remaining.len(), 0);
    }
}
