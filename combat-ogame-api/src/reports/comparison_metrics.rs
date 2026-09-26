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
    for (side, initial, slots, field) in [
        (
            "attacker",
            &input.request.attacker.entities,
            input.request.attacker_slots.as_deref(),
            "units_lost_attackers",
        ),
        (
            "defender",
            &input.request.defender.entities,
            input.request.defender_slots.as_deref(),
            "units_lost_defenders",
        ),
    ] {
        let maps: Vec<_> = samples
            .iter()
            .map(|s| {
                if side == "attacker" {
                    &s.attacker_losses
                } else {
                    &s.defender_losses
                }
            })
            .collect();
        let observed_losses = round_losses(observed, side, None, initial, slots);
        metrics.extend(count_metrics(
            side,
            initial,
            observed_losses.as_ref(),
            Some(&maps),
            samples.len(),
        ));
        metrics.push(numeric(
            &format!("{side}.losses.resources"),
            observed[field].as_u64(),
            maps.iter()
                .map(|m| calculate_losses_value(m, entity_stats()))
                .collect(),
        ));
        for slot in slots.unwrap_or_default() {
            let observed_losses =
                round_losses(observed, side, Some(&slot.id), &slot.data.entities, slots);
            let maps: Option<Vec<_>> = samples
                .iter()
                .map(|s| {
                    let results = if side == "attacker" {
                        s.attacker_slots.as_ref()
                    } else {
                        s.defender_slots.as_ref()
                    }?;
                    let mut matches = results.iter().filter(|r| r.slot_id == slot.id);
                    let result = matches.next()?;
                    if matches.next().is_some() {
                        return None;
                    }
                    Some(&result.losses)
                })
                .collect();
            metrics.extend(count_metrics(
                &slot.id,
                &slot.data.entities,
                observed_losses.as_ref(),
                maps.as_deref(),
                samples.len(),
            ));
        }
    }
    metrics
}

fn count_metrics(
    scope: &str,
    initial: &FleetComposition,
    observed: Option<&BTreeMap<u16, u64>>,
    samples: Option<&[&FleetComposition]>,
    runs: usize,
) -> Vec<MetricComparison> {
    let mut ids: Vec<_> = initial.keys().copied().collect();
    ids.sort_unstable();
    std::iter::once(None).chain(ids.into_iter().map(Some)).map(|entity| {
        let suffix = entity.map_or_else(|| "count".to_owned(), |id| id.to_string());
        let name = format!("{scope}.losses.{suffix}");
        let observation = observed.map(|m| entity.map_or_else(|| m.values().sum(), |id| m.get(&id).copied().unwrap_or(0)));
        let Some(samples) = samples else { return omitted(&name, "participant attribution or simulation losses are unavailable or ambiguous", runs); };
        let mut metric = numeric(&name, observation, samples.iter().map(|m| entity.map_or_else(|| m.values().map(|&n| u64::from(n)).sum(), |id| u64::from(m.get(&id).copied().unwrap_or(0)))).collect());
        if observed.is_none() {
            "complete sequential combat rounds and valid loss counts are required; participant metrics also require unique attribution; absent evidence is not zero".clone_into(&mut metric.explanation);
        }
        metric
    }).collect()
}

fn round_losses(
    observed: &Value,
    side: &str,
    slot: Option<&str>,
    initial: &FleetComposition,
    slots: Option<&[combat_types::PartySlot]>,
) -> Option<BTreeMap<u16, u64>> {
    let count = observed["combat_rounds"].as_u64()?;
    let rounds = observed["rounds"].as_array()?;
    if rounds.len() as u64 != count || count == 0 || count > 6 {
        return None;
    }
    let mut losses = BTreeMap::<u16, u64>::new();
    let mut attributed_losses = BTreeMap::<(&str, u16), u64>::new();
    for (index, round) in rounds.iter().enumerate() {
        if round["round_number"].as_u64()? != index as u64 + 1 {
            return None;
        }
        let entries = round[format!("{side}_ship_losses")].as_array()?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in entries {
            let attributed = entry["slot"].as_str();
            if let Some(id) = attributed {
                if !slots.map_or_else(
                    || id == if side == "attacker" { "A1" } else { "D1" },
                    |slots| slots.iter().any(|s| s.id == id),
                ) {
                    return None;
                }
            } else if slot.is_some() || slots.is_none_or(|s| s.len() == 1) {
                // Preserve the existing single-participant contract: absent
                // ownership is not silently treated as that participant.
                return None;
            }
            let id = u16::try_from(entry["ship_type"].as_u64()?).ok()?;
            if attributed.is_some() && !seen.insert((attributed, id)) {
                return None;
            }
            if let (Some(owner), Some(slots)) = (attributed, slots) {
                let participant = slots.iter().find(|s| s.id == owner)?;
                let total = attributed_losses.entry((owner, id)).or_default();
                *total = total.checked_add(entry["count"].as_u64()?)?;
                if *total > u64::from(*participant.data.entities.get(&id)?) {
                    return None;
                }
            }
            if slot.is_some_and(|slot| attributed != Some(slot)) {
                continue;
            }
            let total = losses.entry(id).or_default();
            *total = total.checked_add(entry["count"].as_u64()?)?;
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
