use combat_ogame_api::reports::{CompletionInput, ReportId, parse_report};
use serde_json::{Value, json};

use crate::support as comparison;

pub fn artifact(owners: [Option<u64>; 4]) -> CompletionInput {
    let participants: Vec<Value> = owners
        .iter()
        .enumerate()
        .map(|(index, owner)| {
            let modified = index % 2 == 1;
            json!({
                "fleet_owner_id":owner, "fleet_owner":"private-name",
                "fleet_weapon_percentage":if modified { 100 } else { 130 },
                "fleet_shield_percentage":if modified { 100 } else { 130 },
                "fleet_armor_percentage":if modified { 100 } else { 130 },
                "fleet_owner_character_class_id":if modified { 0 } else { 2 },
                "fleet_owner_alliance_class_id":if modified { 0 } else { 2 },
                "fleet_composition":[{"ship_type":204,"count":20,
                    "weapon":if modified { 125 } else { 115 },
                    "shield":if modified { 25 } else { 23 },
                    "armor":if modified { 1000 } else { 920 }}]
            })
        })
        .collect();
    let losses = |start: usize| {
        json!([
            {"owner":owners[start],"ship_type":204,"count":2},
            {"owner":owners[start+1],"ship_type":204,"count":3}
        ])
    };
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"winner":"draw","combat_rounds":1,"units_lost_attackers":20000,
            "units_lost_defenders":20000,"debris_metal_total":9000,"debris_crystal_total":3000,"debris_deuterium_total":0},
        "attackers":&participants[..2],"defenders":&participants[2..],
        "rounds":[{"round_number":1,"attacker_ship_losses":losses(0),"defender_ship_losses":losses(2)}]
    }});
    let id = ReportId::parse("cr-en-1-0000000000000000000000000000000000000000").unwrap();
    let candidate = parse_report(&id, &payload.to_string()).unwrap();
    let mut evidence = comparison::evidence();
    for slot in ["A2", "D2"] {
        let mut supplied = evidence.participants["A1"].clone();
        supplied.player_class = Some(combat_types::PlayerClass::None);
        supplied.alliance_class = Some(combat_types::AllianceClass::None);
        supplied.technology.as_mut().unwrap().basis =
            combat_ogame_api::reports::TechnologyBasis::AlreadyEffective;
        supplied.lifeform = Some(
            [(
                204,
                combat_ogame_api::reports::PartialLifeformBonus {
                    weapon: Some(50.0),
                    shield: Some(50.0),
                    armour: Some(50.0),
                    ..Default::default()
                },
            )]
            .into(),
        );
        evidence.participants.insert(slot.into(), supplied);
    }
    CompletionInput {
        candidate,
        evidence,
        universe: comparison::universe(),
    }
}
