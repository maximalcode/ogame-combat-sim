//! Synthetic examples follow the privately verified proxy schema, not real fleets.
use combat_ogame_api::reports::{ReportId, parse_report};
use serde_json::{Value, json};

#[test]
fn espionage_preserves_visibility_and_sanitizes_the_candidate() {
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"sr_id":"private-token","defender_name":"private-name",
            "defender_planet_coordinates":"9:999:9","event_timestamp":1_700_000_000,
            "failed_ships":false,"failed_defense":true,"failed_research":false},
        "details":{"ships":[{"ship_type":204,"count":12}],
            "defense":[{"defense_type":401,"count":999}],
            "research":[{"research_type":109,"level":10},{"research_type":111,"level":12}]}
    }});
    let candidate = parse_report(&id, &payload.to_string()).unwrap();
    let value = serde_json::to_value(&candidate).unwrap();
    assert_eq!(value["defenders"][0]["ships"]["204"], 12);
    assert!(value["defenders"][0]["defenses"].is_null());
    assert!(value["defenders"][0]["entities"].is_null());
    assert_eq!(value["defenders"][0]["technology"]["weapon"], 10);
    assert!(value["defenders"][0]["technology"]["shield"].is_null());
    assert_eq!(value["provenance"]["community"], "en");
    assert_eq!(value["provenance"]["universe"], 1);
    assert_eq!(value["provenance"]["event_timestamp"], 1_700_000_000_u64);
    assert!(value["review_required"].as_array().unwrap().len() >= 3);
    assert!(value["observed"].is_null());
    let output = serde_json::to_string(&candidate).unwrap();
    for private in [
        "private-token",
        "private-name",
        "9:999:9",
        "0000000000000000000000000000000000000000",
    ] {
        assert!(!output.contains(private));
    }
    assert_eq!(
        output,
        serde_json::to_string(&parse_report(&id, &payload.to_string()).unwrap()).unwrap()
    );
}

#[test]
fn invalid_inputs_fail_without_reflecting_private_data() {
    for input in [
        "https://private.example/secret",
        "sr-en-1-../private",
        "sr-en-0-0000000000000000000000000000000000000000",
    ] {
        let error = ReportId::parse(input).unwrap_err();
        assert!(!format!("{error:?} {error}").contains(input));
    }
    assert!(ReportId::parse("rr-en-1-0000000000000000000000000000000000000000").is_err());
    let id = ReportId::parse("cr-en-1-0000000000000000000000000000000000000000").unwrap();
    assert!(!format!("{id:?}").contains("0000000000000000000000000000000000000000"));
    for payload in [
        r#"{"error":{"message":"private-token"}}"#,
        r#"{"RESULT_CODE":6000,"RESULT_DATA":"private-token"}"#,
        r#"{"RESULT_CODE":1000,"RESULT_DATA":{}}"#,
        "private-token",
    ] {
        let error = parse_report(&id, payload).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("private-token"));
    }
    let oversized = " ".repeat(2 * 1024 * 1024 + 1);
    assert!(
        parse_report(&id, &oversized)
            .unwrap_err()
            .to_string()
            .contains("size")
    );
}

#[test]
fn revealed_resources_are_partial_and_unsupported_empty_shapes_fail() {
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"failed_ships":false,"loot_percentage":75},
        "details":{"ships":[],"resources":{"metal":1234,"crystal":0}}}});
    let value = serde_json::to_value(parse_report(&id, &payload.to_string()).unwrap()).unwrap();
    assert_eq!(value["planet_resources"]["metal"], 1234);
    assert_eq!(value["planet_resources"]["crystal"], 0);
    assert!(value["planet_resources"]["deuterium"].is_null());
    assert_eq!(value["loot_percentage"], 75);
    for data in [
        json!({"generic":{},"details":{}}),
        json!({"generic":{},"details":null}),
    ] {
        assert!(
            parse_report(
                &id,
                &json!({"RESULT_CODE":1000,"RESULT_DATA":data}).to_string()
            )
            .is_err()
        );
    }
}

#[test]
fn malformed_fields_cannot_turn_into_plausible_empty_participants() {
    let id = ReportId::parse("cr-en-1-0000000000000000000000000000000000000000").unwrap();
    for participant in [
        Value::Null,
        json!({"fleet_composition":[{"ship_type":204,"count":-1}]}),
        json!({"fleet_composition":[{"ship_type":9999,"count":1}]}),
        json!({"fleet_composition":[{"ship_type":204,"count":1},{"ship_type":204,"count":2}]}),
        json!({"fleet_weapon_percentage":121}),
    ] {
        let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{"generic":{},
            "attackers":[participant],"defenders":[{"fleet_composition":[]}]}});
        assert!(parse_report(&id, &payload.to_string()).is_err());
    }
}

#[test]
fn combat_retains_observations_and_anonymizes_slots_without_reinterpreting_modifiers() {
    let id = ReportId::parse("cr-en-1-0000000000000000000000000000000000000000").unwrap();
    let participant = json!({"fleet_owner_id":987_654,"fleet_owner":"private-name",
        "fleet_weapon_percentage":120,"fleet_shield_percentage":100,"fleet_armor_percentage":110,
        "fleet_owner_character_class_id":2,"fleet_owner_alliance_class_id":1,
        "fleet_composition":[{"ship_type":204,"count":20,"weapon":110,"shield":20,"armor":8400}],
        "lifeformBonuses":{"BaseStatsBooster":{"204":{"weapon":1.2,"shield":1.0,"armor":1.1,"speed":0.5}}}});
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"winner":"attacker","combat_rounds":1,"units_lost_attackers":4000,
            "debris_metal_total":900,"debris_crystal_total":300,"debris_deuterium_total":0},
        "attackers":[participant],"defenders":[{"fleet_owner_id":123_456,
            "fleet_composition":[{"ship_type":401,"count":2}]}],
        "rounds":[{"round_number":1,"statistics":{"attacker_hits":"20"},
            "attacker_ships":[{"owner":987_654,"ship_type":204,"count":19}],
            "attacker_ship_losses":[{"owner":987_654,"ship_type":204,"count":1}],
            "defender_ships":[],"defender_ship_losses":[{"owner":123_456,"ship_type":401,"count":2}]}],
        "repaired_defenses":[{"repaired_type":401,"repaired_count":1}]
    }});
    let candidate = parse_report(&id, &payload.to_string()).unwrap();
    let value = serde_json::to_value(&candidate).unwrap();
    assert_eq!(value["attackers"][0]["entities"]["204"], 20);
    assert_eq!(value["attackers"][0]["technology"]["weapon"], 12);
    assert_eq!(
        value["attackers"][0]["reported_unit_stats"]["204"]["weapon"],
        110.0
    );
    assert_eq!(
        value["attackers"][0]["reported_base_stats_booster"]["204"]["weapon"],
        1.2
    );
    assert_eq!(value["observed"]["winner"], "attacker");
    assert_eq!(
        value["observed"]["rounds"][0]["attacker_ship_losses"][0]["slot"],
        "A1"
    );
    assert_eq!(
        value["observed"]["rounds"][0]["attacker_ship_losses"][0]["count"],
        1
    );
    assert_eq!(value["observed"]["units_lost_attackers"], 4000);
    assert_eq!(value["observed"]["repaired_defenses"]["401"], 1);
    assert!(value["defenders"][0]["technology"]["weapon"].is_null());
    let output = serde_json::to_string(&candidate).unwrap();
    for private in ["987654", "123456", "private-name"] {
        assert!(!output.contains(private));
    }
}

#[test]
fn combat_preserves_moon_outcomes_without_filling_absent_values() {
    let id = ReportId::parse("cr-en-1-0000000000000000000000000000000000000000").unwrap();
    let mut payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"moon_chance":20,"moon_created":true,"moon_exists":false,"moon_size":8000},
        "attackers":[{"fleet_composition":[]}],"defenders":[{"fleet_composition":[]}]}});
    let candidate = serde_json::to_value(parse_report(&id, &payload.to_string()).unwrap()).unwrap();
    assert_eq!(candidate["observed"]["moon_chance"], 20);
    assert_eq!(candidate["observed"]["moon_created"], true);
    assert_eq!(candidate["observed"]["moon_exists"], false);
    assert_eq!(candidate["observed"]["moon_size"], 8000);
    payload["RESULT_DATA"]["generic"] = json!({});
    let candidate = serde_json::to_value(parse_report(&id, &payload.to_string()).unwrap()).unwrap();
    assert!(candidate["observed"]["moon_chance"].is_null());
    assert!(candidate["observed"]["moon_created"].is_null());
    payload["RESULT_DATA"]["generic"] = json!({"moon_created":"private-token"});
    let error = parse_report(&id, &payload.to_string()).unwrap_err();
    assert!(!format!("{error:?} {error}").contains("private-token"));
}

#[test]
fn optional_version_provenance_is_validated_and_never_invented() {
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let mut payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"failed_ships":true,"game_version":"13.0.1"},"details":{}}});
    assert_eq!(
        parse_report(&id, &payload.to_string())
            .unwrap()
            .provenance
            .game_version
            .as_deref(),
        Some("13.0.1")
    );
    payload["RESULT_DATA"]["generic"]["game_version"] = Value::Null;
    assert!(
        parse_report(&id, &payload.to_string())
            .unwrap()
            .provenance
            .game_version
            .is_none()
    );
    for version in [
        json!("private-token"),
        json!("9:999:9"),
        json!(13),
        json!("13..0"),
    ] {
        payload["RESULT_DATA"]["generic"]["game_version"] = version;
        let error = parse_report(&id, &payload.to_string()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid report field provenance.game_version; cannot import this value safely"
        );
    }
}
