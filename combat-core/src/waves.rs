use crate::{Combat, Simulator, simulator::enrich_result};
use combat_types::{WaveSequenceRequest, WaveSequenceResult, WaveSequenceTotals};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rayon::prelude::*;

impl Combat {
    /// Run ordered missions using one caller-owned random stream.
    /// Each battle starts with fresh hulls and shields for the carried survivors.
    /// This uses the same full-size battle and economics as the single-battle path.
    pub fn simulate_waves(
        &self,
        request: &WaveSequenceRequest,
        rng: &mut impl Rng,
    ) -> WaveSequenceResult {
        let mut defender = request.defender.clone();
        let mut resources = request.planet_resources.clone();
        let mut waves = Vec::with_capacity(request.waves.len());
        let mut totals = WaveSequenceTotals::default();
        for wave in &request.waves {
            let battle = self.simulate_single(
                &wave
                    .attacker
                    .at_effective_levels(wave.attacker_bonuses.as_ref()),
                &defender.at_effective_levels(request.defender_bonuses.as_ref()),
                request.use_rapid_fire,
                request.collect_compositions,
                rng,
            );
            let result = enrich_result(battle, resources.as_ref(), request.debris_settings);
            // Retain the input map's iteration order across seeded runs; replacing
            // it with a freshly randomized result map changes target ordering.
            defender.entities.retain(|entity, count| {
                *count = result.defender_remaining.get(entity).copied().unwrap_or(0);
                *count > 0
            });
            if let Some(remaining) = &mut resources {
                remaining.metal -= result.loot.metal;
                remaining.crystal -= result.loot.crystal;
                remaining.deuterium -= result.loot.deuterium;
            }
            totals.rounds += u64::from(result.rounds);
            for (sum, losses) in [
                (&mut totals.attacker_losses, &result.attacker_losses),
                (&mut totals.defender_losses, &result.defender_losses),
            ] {
                for (&entity, &count) in losses {
                    *sum.entry(entity).or_default() += u64::from(count);
                }
            }
            totals.debris_field.metal += result.debris_field.metal;
            totals.debris_field.crystal += result.debris_field.crystal;
            totals.debris_field.deuterium += result.debris_field.deuterium;
            totals.loot.metal += result.loot.metal;
            totals.loot.crystal += result.loot.crystal;
            totals.loot.deuterium += result.loot.deuterium;
            totals.attacker_profit += result.attacker_profit;
            totals.defender_profit += result.defender_profit;
            waves.push(result);
        }
        WaveSequenceResult {
            waves,
            totals,
            final_defender: defender,
            remaining_resources: resources,
        }
    }
}

impl Simulator {
    /// Run independent trajectories in parallel, carrying each run's own survivors.
    /// Zero simulations returns no trajectories; survivors are never averaged
    /// between missions. Use `Combat::simulate_waves` for a seeded trajectory.
    #[must_use]
    pub fn simulate_wave_sequences(
        &self,
        request: &WaveSequenceRequest,
        simulations: u32,
    ) -> Vec<WaveSequenceResult> {
        (0..simulations)
            .into_par_iter()
            .map(|_| {
                let mut rng = SmallRng::from_rng(&mut rand::rng());
                self.combat.simulate_waves(request, &mut rng)
            })
            .collect()
    }
}
