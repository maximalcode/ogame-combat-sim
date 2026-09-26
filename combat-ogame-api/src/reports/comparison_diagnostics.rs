use super::{EvidenceLedger, EvidenceRecord, VerifiedBattleInput};
use combat_core::ModifiedStats;
use combat_types::{Technology, entities::entity_stats};
use serde::Serialize;
use serde_json::Value;
use std::fmt::Write as _;

#[derive(Debug, Serialize)]
pub struct StartingStats {
    pub side: &'static str,
    pub slot: String,
    pub entity: u16,
    pub count: u32,
    pub effective_technology: Technology,
    pub weapon: u32,
    pub shield: f32,
    pub hull: f32,
}

#[derive(Debug, Serialize)]
pub struct ComparisonDiagnostics {
    pub starting_stats: Vec<StartingStats>,
    pub evidence: EvidenceLedger,
    pub model: &'static str,
    pub rapid_fire: bool,
    pub downscaling: Option<bool>,
    pub privacy: &'static str,
}

pub(super) fn diagnostics(input: &VerifiedBattleInput) -> ComparisonDiagnostics {
    let mut starting_stats = Vec::new();
    let mut parties = Vec::new();
    for (side, fallback_id, slots, fallback) in [
        (
            "attacker",
            "A1",
            &input.request.attacker_slots,
            input.request.effective_attacker(),
        ),
        (
            "defender",
            "D1",
            &input.request.defender_slots,
            input.request.effective_defender(),
        ),
    ] {
        if let Some(slots) = slots {
            parties.extend(
                slots
                    .iter()
                    .map(|slot| (side, slot.id.clone(), slot.data.clone())),
            );
        } else {
            parties.push((side, fallback_id.to_owned(), fallback));
        }
    }
    for (side, slot, party) in parties {
        let mut entries: Vec<_> = party.entities.iter().collect();
        entries.sort_unstable_by_key(|(id, _)| **id);
        for (&entity, &count) in entries {
            if let Some(base) = entity_stats().get(&entity) {
                let stats =
                    ModifiedStats::calculate(base, &party.technology, party.lifeform.get(entity));
                starting_stats.push(StartingStats {
                    side,
                    slot: slot.clone(),
                    entity,
                    count,
                    effective_technology: party.technology,
                    weapon: stats.weapon,
                    shield: stats.shield,
                    hull: stats.hull,
                });
            }
        }
    }
    let evidence = EvidenceLedger {
        fields: input
            .evidence
            .fields
            .iter()
            .filter_map(|(path, record)| {
                if !path.split('.').all(known_key) {
                    return None;
                }
                Some((
                    path.clone(),
                    EvidenceRecord {
                        source: record.source,
                        value: safe_value(path.rsplit('.').next().unwrap_or(""), &record.value),
                    },
                ))
            })
            .collect(),
    };
    ComparisonDiagnostics {
        starting_stats,
        evidence,
        model: "combat-core: effective technology plus additive per-entity lifeform percentages; up to six combat rounds; no defence rebuild or wreck-field phase",
        rapid_fire: input.request.use_rapid_fire,
        downscaling: input.request.enable_downscaling,
        privacy: "Private diagnostics. Only recognized evidence fields are included; unrecognized strings are redacted. This is not publication consent or a public regression fixture.",
    }
}

fn known_key(key: &str) -> bool {
    is_slot(key)
        || matches!(
            key,
            "battle"
                | "provenance"
                | "source"
                | "community"
                | "universe"
                | "event_timestamp"
                | "game_version"
                | "source_timestamp"
                | "source_version"
                | "identity"
                | "settings"
                | "current"
                | "acknowledged_current"
                | "historical"
                | "rapid_fire"
                | "galaxies"
                | "systems"
                | "donut_galaxy"
                | "donut_systems"
                | "fleet_speed"
                | "debris_fleet"
                | "debris_defence"
                | "debris_deuterium"
                | "deuterium_save_factor"
                | "entities"
                | "technology"
                | "reported"
                | "basis"
                | "weapon"
                | "shield"
                | "armour"
                | "armor"
                | "cargo"
                | "speed"
                | "lifeform"
                | "player_class"
                | "alliance_class"
                | "reported_unit_stats"
                | "reported_base_stats_booster"
                | "loot_percentage"
        )
        || key
            .parse::<u16>()
            .is_ok_and(|id| entity_stats().contains_key(&id))
}

fn safe_value(key: &str, value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(k, _)| known_key(k))
                .map(|(k, v)| (k.clone(), safe_value(k, v)))
                .collect(),
        ),
        Value::String(s) => {
            let accepted = match key {
                "game_version" | "source_version" => {
                    !s.is_empty()
                        && s.len() <= 32
                        && s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                }
                "community" => s.len() == 2 && s.bytes().all(|b| b.is_ascii_lowercase()),
                "source" => [
                    "report",
                    "public_metadata",
                    "supplied",
                    "community_api_proxy",
                    "synthetic",
                ]
                .contains(&s.as_str()),
                "basis" => [
                    "researched",
                    "already_effective",
                    "reported_combat_bonus_divided_by_ten",
                ]
                .contains(&s.as_str()),
                "player_class" => {
                    ["none", "collector", "general", "discoverer"].contains(&s.as_str())
                }
                "alliance_class" => {
                    ["none", "warrior", "trader", "researcher"].contains(&s.as_str())
                }
                _ => false,
            };
            if accepted {
                value.clone()
            } else {
                Value::String("[redacted]".to_owned())
            }
        }
        Value::Array(_) => Value::Null,
        _ => value.clone(),
    }
}

fn is_slot(key: &str) -> bool {
    let Some(number) = key.strip_prefix('A').or_else(|| key.strip_prefix('D')) else {
        return false;
    };
    number
        .parse::<usize>()
        .is_ok_and(|n| n > 0 && n.to_string() == number)
}

impl VerifiedBattleInput {
    /// Sanitized participant inputs and evidence for private local inspection.
    #[must_use]
    pub fn diagnostics(&self) -> ComparisonDiagnostics {
        diagnostics(self)
    }
}

impl ComparisonDiagnostics {
    /// Render each verified participant without conflating their modifiers.
    #[must_use]
    pub fn render_text(&self) -> String {
        let mut output = String::new();
        for stats in &self.starting_stats {
            let _ = writeln!(
                output,
                "Participant {} ({}): entity {} x {}; effective W/S/A {}/{}/{}; weapon {}, shield {}, hull {}",
                stats.slot,
                stats.side,
                stats.entity,
                stats.count,
                stats.effective_technology.weapon,
                stats.effective_technology.shield,
                stats.effective_technology.armour,
                stats.weapon,
                stats.shield,
                stats.hull
            );
        }
        let _ = writeln!(
            output,
            "{}",
            serde_json::to_string_pretty(self).unwrap_or_default()
        );
        output
    }
}
