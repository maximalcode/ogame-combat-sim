//! Evidence-to-metric mapping. Missing or ambiguous evidence stays unassessable.
use super::VerifiedBattleInput;
use super::comparison::{MetricComparison, numeric, omitted};
use combat_core::economics::calculate_losses_value;
use combat_types::{FleetComposition, SimulationResult, entities::entity_stats};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn losses(
    input: &VerifiedBattleInput,
    samples: &[SimulationResult],
) -> Vec<MetricComparison> {
    let observed = input.observed.as_ref().unwrap_or(&Value::Null);
    let mut metrics = Vec::new();
    for (side, slot, initial, field) in [
        (
            "attacker",
            "A1",
            &input.request.attacker.entities,
            "units_lost_attackers",
        ),
        (
            "defender",
            "D1",
            &input.request.defender.entities,
            "units_lost_defenders",
        ),
    ] {
        let maps: Vec<&FleetComposition> = samples
            .iter()
            .map(|s| {
                if side == "attacker" {
                    &s.attacker_losses
                } else {
                    &s.defender_losses
                }
            })
            .collect();
        let observed_losses = round_losses(observed, side, slot, initial);
        let total = observed_losses.as_ref().map(|m| m.values().sum());
        let first_count_metric = metrics.len();
        metrics.push(numeric(
            &format!("{side}.losses.count"),
            total,
            maps.iter()
                .map(|m| m.values().map(|&v| u64::from(v)).sum())
                .collect(),
        ));
        let mut ids: Vec<_> = initial.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            let count = observed_losses
                .as_ref()
                .map(|m| m.get(&id).copied().unwrap_or(0));
            metrics.push(numeric(
                &format!("{side}.losses.{id}"),
                count,
                maps.iter()
                    .map(|m| u64::from(m.get(&id).copied().unwrap_or(0)))
                    .collect(),
            ));
        }
        if observed_losses.is_none() {
            for metric in &mut metrics[first_count_metric..] {
                "complete sequential combat rounds with explicit loss arrays and unique participant attribution are required; absent losses are not zero".clone_into(&mut metric.explanation);
            }
        }
        metrics.push(numeric(
            &format!("{side}.losses.resources"),
            observed[field].as_u64(),
            maps.iter()
                .map(|m| calculate_losses_value(m, entity_stats()))
                .collect(),
        ));
    }
    metrics
}

fn round_losses(
    observed: &Value,
    side: &str,
    slot: &str,
    initial: &FleetComposition,
) -> Option<BTreeMap<u16, u64>> {
    let count = observed["combat_rounds"].as_u64()?;
    let rounds = observed["rounds"].as_array()?;
    if rounds.len() as u64 != count || count == 0 || count > 6 {
        return None;
    }
    let mut losses = BTreeMap::<u16, u64>::new();
    for (index, round) in rounds.iter().enumerate() {
        if round["round_number"].as_u64()? != index as u64 + 1 {
            return None;
        }
        let entries = round[format!("{side}_ship_losses")].as_array()?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in entries {
            if entry["slot"].as_str()? != slot {
                return None;
            }
            let id = u16::try_from(entry["ship_type"].as_u64()?).ok()?;
            if !seen.insert(id) {
                return None;
            }
            let count = entry["count"].as_u64()?;
            let total = losses.entry(id).or_default();
            *total = total.checked_add(count)?;
            if *total > u64::from(*initial.get(&id)?) {
                return None;
            }
        }
    }
    Some(losses)
}

pub(super) fn debris(
    input: &VerifiedBattleInput,
    samples: &[SimulationResult],
) -> Vec<MetricComparison> {
    let observed = input.observed.as_ref().unwrap_or(&Value::Null);
    let limited = input
        .assessment_limitations
        .iter()
        .any(|l| l.metric == "generated_debris");
    let mut metrics = Vec::new();
    for resource in ["metal", "crystal", "deuterium"] {
        let name = format!("generated_debris.{resource}");
        let observation = observed[format!("debris_{resource}_total")].as_u64();
        let mut metric = if limited {
            omitted(
                &name,
                "battle-time debris settings are missing or only acknowledged as current; generated debris is not assessable",
                samples.len(),
            )
        } else {
            numeric(
                &name,
                observation,
                samples
                    .iter()
                    .map(|s| match resource {
                        "metal" => s.debris_field.metal,
                        "crystal" => s.debris_field.crystal,
                        _ => s.debris_field.deuterium,
                    })
                    .collect(),
            )
        };
        metric.observation = observation.map_or(Value::Null, Value::from);
        metrics.push(metric);
        let mut remaining = omitted(
            &format!("remaining_debris.{resource}"),
            "debris remaining after collection is not total generated debris",
            samples.len(),
        );
        remaining.observation = observed[format!("debris_{resource}")]
            .as_u64()
            .map_or(Value::Null, Value::from);
        metrics.push(remaining);
    }
    metrics
}

/// Only quantities keyed by known entity IDs are retained for unsupported phases.
pub(super) fn numeric_evidence(value: &Value) -> Value {
    if let Some(n) = value.as_u64() {
        return n.into();
    }
    value.as_object().map_or(Value::Null, |map| {
        Value::Object(
            map.iter()
                .filter_map(|(key, value)| {
                    let id = key.parse::<u16>().ok()?;
                    entity_stats().contains_key(&id).then_some(())?;
                    Some((id.to_string(), value.as_u64()?.into()))
                })
                .collect(),
        )
    })
}
