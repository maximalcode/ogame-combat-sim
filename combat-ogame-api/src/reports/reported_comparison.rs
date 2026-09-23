//! Conditional combat-resolution diagnostics, distinct from verified completion.
use super::comparison::{assess_probability, bounded_samples, numeric, omitted};
use super::{ComparisonStage, MetricComparison};
use combat_core::{Combat, ReportedBattle, ReportedSlot, RoundOutcome, SingleCombatResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RapidFireBasis {
    Reported,
    Assumed,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservedOutcome {
    Attacker,
    Defender,
    Draw,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedSource {
    pub archive_report: Option<u64>,
    pub battle_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedObservation {
    pub winner: Option<ObservedOutcome>,
    pub rounds: Option<u8>,
    /// Explicit final pre-repair counts, one map per opening slot. Missing
    /// maps mean unknown. Every starting entity must be present, including zeros.
    pub attacker_remaining: Option<Vec<BTreeMap<u16, u32>>>,
    pub defender_remaining: Option<Vec<BTreeMap<u16, u32>>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedComparisonInput {
    pub battle: ReportedBattle,
    pub rapid_fire_basis: RapidFireBasis,
    pub source: ReportedSource,
    pub observed: ReportedObservation,
}

#[derive(Debug, Serialize)]
pub struct ReportedComparison {
    pub run_count: usize,
    pub stages: Vec<ComparisonStage>,
    pub metrics: Vec<MetricComparison>,
    pub input: ReportedComparisonInput,
    pub method: &'static str,
    pub limitations: &'static str,
    pub software_version: &'static str,
}

impl ReportedComparisonInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.battle.validate()?;
        if self.observed.rounds.is_some_and(|n| n > 6) {
            return Err("observed round count must be within 0..6");
        }
        for (side, observation) in [
            (&self.battle.attackers, &self.observed.attacker_remaining),
            (&self.battle.defenders, &self.observed.defender_remaining),
        ] {
            if let Some(remaining) = observation {
                if remaining.len() != side.len() {
                    return Err("observed final slots must match opening slots");
                }
                for (slot, counts) in side.iter().zip(remaining) {
                    if counts.len() != slot.units.len()
                        || slot
                            .units
                            .iter()
                            .any(|(id, u)| counts.get(id).is_none_or(|n| *n > u.count))
                    {
                        return Err(
                            "observed final counts must include each opening entity and cannot exceed its count",
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

pub fn compare_reported(
    input: &ReportedComparisonInput,
) -> Result<ReportedComparison, &'static str> {
    input.validate()?;
    compare_reported_with(input, |n| {
        (0..n)
            .map(|_| {
                Combat::new()
                    .simulate_reported(&input.battle)
                    .expect("validated reported battle")
            })
            .collect()
    })
}

/// Controlled samples use the same assessment and cumulative policy as real runs.
pub fn compare_reported_with(
    input: &ReportedComparisonInput,
    mut simulate: impl FnMut(usize) -> Vec<SingleCombatResult>,
) -> Result<ReportedComparison, &'static str> {
    input.validate()?;
    let sampling = bounded_samples(|n| Ok(simulate(n)), |samples| assess(input, samples))?;
    Ok(ReportedComparison {
        run_count: sampling.run_count,
        stages: sampling.stages,
        metrics: sampling.metrics,
        input: input.clone(),
        method: "Inclusive empirical two-sided tails with ties; doubled/capped 95% Wilson intervals for numeric metrics, ordinary Wilson occurrence intervals for outcome; 5% rarity boundary. Cumulative 50, 200, 1000 runs; no multiple-testing adjustment or global pass. Existing combat-core slot round loop and rapid-fire table; no downscaling.",
        limitations: "Conditional on supplied reported weapon/shield/hull values and the stated rapid-fire setting/basis. Numeric validation only: report precision and modifier provenance are unverified. This does not validate API import, technology, classes, lifeforms or historical universe rules. No defence rebuild, wreck-field or special General perk modeling. Public archive evidence is not publication consent.",
        software_version: env!("CARGO_PKG_VERSION"),
    })
}

fn assess(
    input: &ReportedComparisonInput,
    samples: &[SingleCombatResult],
) -> Vec<MetricComparison> {
    let mut outcome = omitted("outcome", "no observed outcome", samples.len());
    if let Some(winner) = input.observed.winner {
        outcome.observation = serde_json::to_value(winner).expect("enum serialization");
        let expected = match winner {
            ObservedOutcome::Attacker => RoundOutcome::AttackersWin,
            ObservedOutcome::Defender => RoundOutcome::DefendersWin,
            ObservedOutcome::Draw => RoundOutcome::Draw,
        };
        if !samples.is_empty() {
            assess_probability(
                &mut outcome,
                samples.iter().filter(|s| s.outcome == expected).count(),
                1.0,
            );
        }
    }
    let mut metrics = vec![
        outcome,
        numeric(
            "rounds",
            input.observed.rounds.map(u64::from),
            samples.iter().map(|s| u64::from(s.rounds)).collect(),
        ),
    ];
    for (side, slots, observed) in [
        (
            "attacker",
            &input.battle.attackers,
            &input.observed.attacker_remaining,
        ),
        (
            "defender",
            &input.battle.defenders,
            &input.observed.defender_remaining,
        ),
    ] {
        losses(&mut metrics, side, slots, observed.as_deref(), samples);
    }
    for name in [
        "debris_metal",
        "debris_crystal",
        "debris_deuterium",
        "loot",
        "repaired_defenses",
        "wreck_field",
        "moon_chance",
        "round_firepower",
        "round_shield_absorption",
    ] {
        metrics.push(omitted(name,"outside this conditional outcome/round-count/loss comparison; matching historical settings or supported observation semantics are not established",samples.len()));
    }
    metrics
}

fn losses(
    metrics: &mut Vec<MetricComparison>,
    side: &str,
    slots: &[ReportedSlot],
    observed: Option<&[BTreeMap<u16, u32>]>,
    samples: &[SingleCombatResult],
) {
    let prefix = if side == "attacker" { "A" } else { "D" };
    let mut attribution_complete = true;
    let mut aggregate_observed = 0u64;
    let mut aggregate_samples = vec![0u64; samples.len()];
    let mut costs_observed = 0u64;
    let mut costs_samples = vec![0u64; samples.len()];
    for (index, slot) in slots.iter().enumerate() {
        let slot_id = format!("{prefix}{}", index + 1);
        for (&id, unit) in &slot.units {
            let observation = observed.map(|counts| u64::from(unit.count - counts[index][&id]));
            let values: Option<Vec<u64>> = samples
                .iter()
                .map(|s| {
                    let results = if side == "attacker" {
                        &s.attacker_slots
                    } else {
                        &s.defender_slots
                    };
                    results
                        .as_ref()?
                        .iter()
                        .find(|s| s.slot_id == slot_id)
                        .map(|s| u64::from(s.losses.get(&id).copied().unwrap_or(0)))
                })
                .collect();
            let name = format!("{slot_id}.losses.{id}");
            let Some(values) = values else {
                attribution_complete = false;
                metrics.push(omitted(
                    &name,
                    "simulation has no matching slot attribution",
                    samples.len(),
                ));
                continue;
            };
            let base = &combat_types::entities::entity_stats()[&id];
            let cost = u64::from(base.cost_metal)
                + u64::from(base.cost_crystal)
                + u64::from(base.cost_deuterium);
            aggregate_observed += observation.unwrap_or(0);
            costs_observed += observation.unwrap_or(0) * cost;
            for ((total, cost_total), value) in aggregate_samples
                .iter_mut()
                .zip(&mut costs_samples)
                .zip(&values)
            {
                *total += value;
                *cost_total += value * cost;
            }
            metrics.push(numeric(&name, observation, values));
        }
    }
    if !attribution_complete {
        for name in ["loss_count", "resource_losses"] {
            metrics.push(omitted(
                &format!("{side}.{name}"),
                "simulation has incomplete slot attribution",
                samples.len(),
            ));
        }
        return;
    }
    metrics.push(numeric(
        &format!("{side}.loss_count"),
        observed.map(|_| aggregate_observed),
        aggregate_samples,
    ));
    metrics.push(numeric(
        &format!("{side}.resource_losses"),
        observed.map(|_| costs_observed),
        costs_samples,
    ));
}
