use super::{EvidenceLedger, EvidenceRecord, VerifiedBattleInput};
use combat_core::ModifiedStats;
use combat_types::{Technology, entities::entity_stats};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct StartingStats {
    pub side: &'static str,
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
    for (side, party) in [
        ("attacker", input.request.effective_attacker()),
        ("defender", input.request.effective_defender()),
    ] {
        let mut entries: Vec<_> = party.entities.iter().collect();
        entries.sort_unstable_by_key(|(id, _)| **id);
        for (&entity, &count) in entries {
            if let Some(base) = entity_stats().get(&entity) {
                let stats =
                    ModifiedStats::calculate(base, &party.technology, party.lifeform.get(entity));
                starting_stats.push(StartingStats {
                    side,
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
    matches!(
        key,
        "A1" | "D1"
            | "battle"
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
            | "cargo"
            | "speed"
            | "lifeform"
            | "player_class"
            | "alliance_class"
            | "reported_unit_stats"
            | "reported_base_stats_booster"
            | "loot_percentage"
    ) || key
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
