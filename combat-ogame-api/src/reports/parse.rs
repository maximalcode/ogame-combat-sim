use super::model::Composition;
use super::{Candidate, Participant, Provenance, ReportError, ReportId, TechnologyCandidate};
use serde_json::Value;

/// Parse a captured full proxy envelope offline; unknown fields are discarded.
/// Only allowlisted typed values cross into the sanitized candidate.
pub fn parse_report(id: &ReportId, json: &str) -> Result<Candidate, ReportError> {
    if json.len() > super::MAX_REPORT_BYTES {
        return Err(ReportError::TooLarge);
    }
    let root: Value = serde_json::from_str(json).map_err(|_| ReportError::Malformed)?;
    if root.get("error").is_some() {
        return Err(ReportError::Provider);
    }
    match root.get("RESULT_CODE").and_then(Value::as_u64) {
        Some(1000) => {}
        Some(_) => return Err(ReportError::Provider),
        None => return Err(ReportError::Malformed),
    }
    let data = root
        .get("RESULT_DATA")
        .filter(|v| v.is_object())
        .ok_or(ReportError::Malformed)?;
    let generic = data
        .get("generic")
        .filter(|v| v.is_object())
        .ok_or(ReportError::Malformed)?;
    let mut candidate = Candidate {
        schema_version: 1,
        report_kind: id.kind,
        provenance: Provenance {
            source: "community_api_proxy".to_owned(),
            community: id.community.clone(),
            universe: id.universe,
            event_timestamp: number(generic, "event_timestamp", "provenance.event_timestamp")?,
            game_version: game_version(generic)?,
        },
        attackers: Vec::new(),
        defenders: Vec::new(),
        observed: None,
        planet_resources: None,
        loot_percentage: number(generic, "loot_percentage", "loot_percentage")?
            .map(|v| {
                u8::try_from(v)
                    .ok()
                    .filter(|v| *v <= 100)
                    .ok_or_else(|| ReportError::Field("loot_percentage".to_owned()))
            })
            .transpose()?,
        review_required: vec![
            "universe_settings: supply debris rules and review simulation settings".to_owned(),
        ],
    };
    match id.kind {
        super::ReportKind::Combat => combat(data, generic, &mut candidate)?,
        super::ReportKind::Espionage => espionage(data, generic, &mut candidate)?,
    }
    Ok(candidate)
}

// Optional compatibility field, absent from the inspected proxy samples.
// A restricted dotted-numeric form prevents arbitrary provider text leaking
// through provenance; no version is inferred from a timestamp or universe.
fn game_version(generic: &Value) -> Result<Option<String>, ReportError> {
    let Some(value) = generic.get("game_version").filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let invalid = || ReportError::Field("provenance.game_version".to_owned());
    let version = value
        .as_str()
        .filter(|v| v.len() <= 32)
        .ok_or_else(invalid)?;
    let segments: Vec<_> = version.split('.').collect();
    if !(2..=4).contains(&segments.len())
        || segments
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(invalid());
    }
    Ok(Some(version.to_owned()))
}

fn espionage(data: &Value, generic: &Value, candidate: &mut Candidate) -> Result<(), ReportError> {
    if !["failed_ships", "failed_defense", "failed_research"]
        .iter()
        .any(|key| generic.get(key).is_some())
    {
        return Err(ReportError::Malformed);
    }
    let details = data
        .get("details")
        .filter(|v| v.is_object())
        .ok_or(ReportError::Malformed)?;
    candidate.planet_resources = details
        .get("resources")
        .filter(|v| !v.is_null())
        .map(|resources| {
            if !resources.is_object() {
                return Err(ReportError::Field("planet_resources".to_owned()));
            }
            Ok(super::ResourcesCandidate {
                metal: number(resources, "metal", "planet_resources.metal")?,
                crystal: number(resources, "crystal", "planet_resources.crystal")?,
                deuterium: number(resources, "deuterium", "planet_resources.deuterium")?,
            })
        })
        .transpose()?;
    let ships = visible_composition(generic, details, "failed_ships", "ships", "ship_type")?;
    let defenses = visible_composition(
        generic,
        details,
        "failed_defense",
        "defense",
        "defense_type",
    )?;
    let entities = match (&ships, &defenses) {
        (Some(ships), Some(defenses)) => {
            let mut combined = ships.clone();
            for (&kind, &count) in defenses {
                if combined.insert(kind, count).is_some() {
                    return Err(ReportError::Field(
                        "defenders.entities: duplicate entity".to_owned(),
                    ));
                }
            }
            Some(combined)
        }
        _ => None,
    };
    let technology = espionage_technology(generic, details)?;
    candidate
        .review_required
        .push("attackers: supply an attacking fleet and its modifiers".to_owned());
    candidate.review_required.push(
        "defenders.modifiers: verify class and lifeform modifiers before simulation".to_owned(),
    );
    if entities.is_none() {
        candidate
            .review_required
            .push("defenders.entities: ships or defenses were not revealed".to_owned());
    }
    if technology.weapon.is_none() || technology.shield.is_none() || technology.armour.is_none() {
        candidate.review_required.push(
            "defenders.technology: supply missing weapon, shield or armour levels".to_owned(),
        );
    }
    candidate.defenders.push(Participant {
        slot: "D1".to_owned(),
        ships,
        defenses,
        entities,
        technology,
        character_class_id: class(generic, "defender_character_class_id")?,
        alliance_class_id: class(generic, "defender_alliance_class_id")?,
        reported_base_stats_booster: boosters(details)?,
        reported_unit_stats: None,
    });
    Ok(())
}

fn espionage_technology(
    generic: &Value,
    details: &Value,
) -> Result<TechnologyCandidate, ReportError> {
    let mut technology = TechnologyCandidate {
        basis: "researched".to_owned(),
        ..Default::default()
    };
    if optional_bool(generic, "failed_research")? == Some(false) {
        if let Some(research) = optional_array(details, "research")? {
            for entry in research {
                let kind = number(entry, "research_type", "defenders.technology.research_type")?
                    .ok_or(ReportError::Malformed)?;
                let target = match kind {
                    109 => &mut technology.weapon,
                    110 => &mut technology.shield,
                    111 => &mut technology.armour,
                    _ => continue,
                };
                if target.is_some() {
                    return Err(ReportError::Field(
                        "defenders.technology: duplicate research".to_owned(),
                    ));
                }
                *target = number(entry, "level", "defenders.technology.level")?
                    .map(u8::try_from)
                    .transpose()
                    .map_err(|_| ReportError::Field("defenders.technology.level".to_owned()))?;
            }
        }
    }
    Ok(technology)
}

fn combat(data: &Value, generic: &Value, candidate: &mut Candidate) -> Result<(), ReportError> {
    let mut owners = Vec::new();
    for (field, prefix, target) in [
        ("attackers", "A", &mut candidate.attackers),
        ("defenders", "D", &mut candidate.defenders),
    ] {
        let participants = optional_array(data, field)?.ok_or(ReportError::Malformed)?;
        if participants.is_empty() {
            return Err(ReportError::Malformed);
        }
        for (index, participant) in participants.iter().enumerate() {
            let slot = format!("{prefix}{}", index + 1);
            let owner = number(participant, "fleet_owner_id", "participant.owner")?;
            owners.push((prefix, owner, slot.clone()));
            let fleet = optional_array(participant, "fleet_composition")?;
            let entities = fleet
                .map(|v| composition(v, "ship_type", "count", field))
                .transpose()?;
            let technology = TechnologyCandidate {
                basis: "reported_combat_bonus_divided_by_ten".to_owned(),
                weapon: combat_level(participant, "fleet_weapon_percentage")?,
                shield: combat_level(participant, "fleet_shield_percentage")?,
                armour: combat_level(participant, "fleet_armor_percentage")?,
            };
            if entities.is_none() {
                candidate
                    .review_required
                    .push(format!("{slot}.entities: supply missing composition"));
            }
            if technology.weapon.is_none()
                || technology.shield.is_none()
                || technology.armour.is_none()
            {
                candidate
                    .review_required
                    .push(format!("{slot}.technology: supply missing levels"));
            }
            candidate.review_required.push(format!("{slot}.modifiers: verify reported technology/class treatment and BaseStatsBooster units; do not apply them twice"));
            target.push(Participant {
                slot,
                entities,
                ships: None,
                defenses: None,
                technology,
                character_class_id: class(participant, "fleet_owner_character_class_id")?,
                alliance_class_id: class(participant, "fleet_owner_alliance_class_id")?,
                reported_base_stats_booster: boosters(participant)?,
                reported_unit_stats: fleet
                    .map(|items| unit_stats(items, "ship_type"))
                    .transpose()?,
            });
        }
    }
    candidate.observed = Some(observations(data, generic, &owners)?);
    Ok(())
}

fn observations(
    data: &Value,
    generic: &Value,
    owners: &[(&str, Option<u64>, String)],
) -> Result<Value, ReportError> {
    let mut observed = serde_json::Map::new();
    let winner = match generic.get("winner") {
        None | Some(Value::Null) => Value::Null,
        Some(Value::String(value))
            if ["attacker", "defender", "draw"].contains(&value.as_str()) =>
        {
            Value::String(value.clone())
        }
        _ => return Err(ReportError::Field("observed.winner".to_owned())),
    };
    observed.insert("winner".to_owned(), winner);
    for field in [
        "combat_rounds",
        "units_lost_attackers",
        "units_lost_defenders",
        "debris_metal_total",
        "debris_crystal_total",
        "debris_deuterium_total",
        "debris_metal",
        "debris_crystal",
        "debris_deuterium",
        "loot_metal",
        "loot_crystal",
        "loot_deuterium",
        "moon_chance",
        "moon_size",
    ] {
        observed.insert(
            field.to_owned(),
            number(generic, field, field)?.map_or(Value::Null, Value::from),
        );
    }
    for field in ["moon_created", "moon_exists"] {
        observed.insert(
            field.to_owned(),
            optional_bool(generic, field)?.map_or(Value::Null, Value::from),
        );
    }
    observed.insert(
        "repaired_defenses".to_owned(),
        match optional_array(data, "repaired_defenses")? {
            Some(items) => serde_json::to_value(composition(
                items,
                "repaired_type",
                "repaired_count",
                "observed.repaired_defenses",
            )?)
            .map_err(|_| ReportError::Malformed)?,
            None => Value::Null,
        },
    );
    observed.insert(
        "rounds".to_owned(),
        match optional_array(data, "rounds")? {
            Some(rounds) => Value::Array(
                rounds
                    .iter()
                    .map(|round| observed_round(round, owners))
                    .collect::<Result<_, _>>()?,
            ),
            None => Value::Null,
        },
    );
    Ok(Value::Object(observed))
}

fn observed_round(
    round: &Value,
    owners: &[(&str, Option<u64>, String)],
) -> Result<Value, ReportError> {
    let mut result = serde_json::Map::new();
    result.insert(
        "round_number".to_owned(),
        number(round, "round_number", "observed.rounds.round_number")?
            .map_or(Value::Null, Value::from),
    );
    for (field, side) in [
        ("attacker_ships", "A"),
        ("attacker_ship_losses", "A"),
        ("defender_ships", "D"),
        ("defender_ship_losses", "D"),
    ] {
        let entries = optional_array(round, field)?
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let owner = number(item, "owner", "observed.rounds.owner")?;
                        let matches: Vec<_> = owners
                            .iter()
                            .filter(|(prefix, key, _)| {
                                *prefix == side && owner.is_some() && *key == owner
                            })
                            .collect();
                        // Multiple ACS fleets may share one owner. Do not invent a slot association.
                        let slot = if matches.len() == 1 {
                            Some(matches[0].2.as_str())
                        } else {
                            None
                        };
                        let fleet = composition(
                            std::slice::from_ref(item),
                            "ship_type",
                            "count",
                            "observed.rounds",
                        )?;
                        let (&ship_type, &count) =
                            fleet.first_key_value().ok_or(ReportError::Malformed)?;
                        Ok(serde_json::json!({"slot":slot,"ship_type":ship_type,"count":count}))
                    })
                    .collect::<Result<Vec<_>, ReportError>>()
            })
            .transpose()?;
        result.insert(field.to_owned(), entries.map_or(Value::Null, Value::Array));
    }
    let mut stats = serde_json::Map::new();
    if let Some(statistics) = round.get("statistics").filter(|v| !v.is_null()) {
        if !statistics.is_object() {
            return Err(ReportError::Field("observed.rounds.statistics".to_owned()));
        }
        for field in [
            "attacker_hits",
            "attacker_absorbed",
            "attacker_fullstrength",
            "defender_hits",
            "defender_absorbed",
            "defender_fullstrength",
        ] {
            let value = match statistics.get(field) {
                Some(Value::String(value)) => Some(value.parse::<u64>().map_err(|_| {
                    ReportError::Field(format!("observed.rounds.statistics.{field}"))
                })?),
                _ => number(statistics, field, field)?,
            };
            stats.insert(field.to_owned(), value.map_or(Value::Null, Value::from));
        }
    }
    result.insert(
        "statistics".to_owned(),
        if stats.is_empty() {
            Value::Null
        } else {
            Value::Object(stats)
        },
    );
    Ok(Value::Object(result))
}

fn combat_level(participant: &Value, key: &str) -> Result<Option<u8>, ReportError> {
    number(participant, key, key)?
        .map(|value| {
            if value % 10 != 0 {
                return Err(ReportError::Field(key.to_owned()));
            }
            u8::try_from(value / 10).map_err(|_| ReportError::Field(key.to_owned()))
        })
        .transpose()
}

fn class(object: &Value, key: &str) -> Result<Option<u8>, ReportError> {
    number(object, key, key)?
        .map(|value| {
            u8::try_from(value)
                .ok()
                .filter(|v| *v <= 3)
                .ok_or_else(|| ReportError::Field(key.to_owned()))
        })
        .transpose()
}

fn stats(object: &Value) -> Result<Value, ReportError> {
    if !object.is_object() {
        return Err(ReportError::Field("reported_stats".to_owned()));
    }
    let mut result = serde_json::Map::new();
    for field in ["weapon", "shield", "armor", "cargo", "speed"] {
        let value = match object.get(field) {
            None | Some(Value::Null) => Value::Null,
            Some(value) => {
                let number = value
                    .as_f64()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(|| ReportError::Field(format!("reported_stats.{field}")))?;
                Value::from(number)
            }
        };
        result.insert(field.to_owned(), value);
    }
    Ok(Value::Object(result))
}

fn boosters(object: &Value) -> Result<Option<Value>, ReportError> {
    let Some(lifeform) = object.get("lifeformBonuses").filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    if !lifeform.is_object() {
        return Err(ReportError::Field("reported_base_stats_booster".to_owned()));
    }
    let Some(boosters) = lifeform.get("BaseStatsBooster").filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let boosters = boosters
        .as_object()
        .ok_or_else(|| ReportError::Field("reported_base_stats_booster".to_owned()))?;
    let mut result = serde_json::Map::new();
    for (key, value) in boosters {
        let kind = key
            .parse::<u16>()
            .ok()
            .filter(|v| combat_types::names::name_of(*v).is_some())
            .ok_or_else(|| {
                ReportError::Field("reported_base_stats_booster.entity_type".to_owned())
            })?;
        if result.insert(kind.to_string(), stats(value)?).is_some() {
            return Err(ReportError::Field(
                "reported_base_stats_booster: duplicate entity".to_owned(),
            ));
        }
    }
    Ok(Some(Value::Object(result)))
}

fn unit_stats(items: &[Value], key: &str) -> Result<Value, ReportError> {
    let mut result = serde_json::Map::new();
    for item in items {
        let kind =
            number(item, key, "reported_unit_stats.entity_type")?.ok_or(ReportError::Malformed)?;
        result.insert(kind.to_string(), stats(item)?);
    }
    Ok(Value::Object(result))
}

fn visible_composition(
    generic: &Value,
    details: &Value,
    flag: &str,
    field: &str,
    kind: &str,
) -> Result<Option<Composition>, ReportError> {
    if optional_bool(generic, flag)? != Some(false) {
        return Ok(None);
    }
    optional_array(details, field)?
        .map(|items| composition(items, kind, "count", field))
        .transpose()
}

fn optional_bool(object: &Value, key: &str) -> Result<Option<bool>, ReportError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| ReportError::Field(key.to_owned())),
    }
}

fn optional_array<'a>(object: &'a Value, key: &str) -> Result<Option<&'a Vec<Value>>, ReportError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_array()
            .map(Some)
            .ok_or_else(|| ReportError::Field(key.to_owned())),
    }
}

fn number(object: &Value, key: &str, path: &str) -> Result<Option<u64>, ReportError> {
    if !object.is_object() {
        return Err(ReportError::Field(path.to_owned()));
    }
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| ReportError::Field(path.to_owned())),
    }
}

fn composition(
    items: &[Value],
    type_key: &str,
    count_key: &str,
    path: &str,
) -> Result<Composition, ReportError> {
    let mut result = Composition::new();
    for item in items {
        let kind = number(item, type_key, path)?
            .and_then(|v| u16::try_from(v).ok())
            .filter(|v| combat_types::names::name_of(*v).is_some())
            .ok_or_else(|| ReportError::Field(format!("{path}.entity_type")))?;
        let count = number(item, count_key, path)?
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| ReportError::Field(format!("{path}.count")))?;
        if result.insert(kind, count).is_some() {
            return Err(ReportError::Field(format!("{path}: duplicate entity")));
        }
    }
    Ok(result)
}
