use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn report_import_requires_explicit_transfer_consent_and_redacts_bad_ids() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .arg("report")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("third-party"));
    assert!(error.contains("cach"));
    let mut child = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "--allow-proxy-transfer"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"https://private.example/secret\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("private.example")
    );
}

#[test]
fn mistaken_positional_ids_are_not_echoed_by_argument_errors() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "private-token"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("private-token")
    );
}

#[test]
fn report_complete_renders_the_shared_library_result() {
    let path =
        std::env::temp_dir().join(format!("combat-cli-completion-{}.json", std::process::id()));
    let artifact = serde_json::json!({
        "candidate": {
            "schema_version": 1,
            "report_kind": "combat",
            "provenance": {"source":"synthetic","community":"en","universe":1,
                "event_timestamp":1_700_000_000,"game_version":"13.0.1"},
            "attackers": [{"slot":"A1","entities":{"204":20},"ships":null,"defenses":null,
                "technology":{"basis":"reported_combat_bonus_divided_by_ten","weapon":13,"shield":13,"armour":13},
                "character_class_id":2,"alliance_class_id":2,
                "reported_base_stats_booster":{"204":{"weapon":1.2}},"reported_unit_stats":null}],
            "defenders": [{"slot":"D1","entities":{"401":2},"ships":null,"defenses":null,
                "technology":{"basis":"reported_combat_bonus_divided_by_ten","weapon":13,"shield":13,"armour":13},
                "character_class_id":2,"alliance_class_id":2,
                "reported_base_stats_booster":null,"reported_unit_stats":null}],
            "observed":{"winner":"attacker"},"planet_resources":null,"loot_percentage":50,
            "review_required":[]
        },
        "evidence": {"participants": {
            "A1":{"technology":{"basis":"researched","weapon":10,"shield":10,"armour":10},"player_class":"general","alliance_class":"warrior","lifeform":{}},
            "D1":{"technology":{"basis":"researched","weapon":10,"shield":10,"armour":10},"player_class":"general","alliance_class":"warrior","lifeform":{}}
        }},
        "universe": {"community":"en","universe":1,
            "settings":{"galaxies":9,"systems":499,"donut_galaxy":true,"donut_systems":true,"fleet_speed":1,
                "debris_fleet":30,"debris_defence":0,"debris_deuterium":false,"deuterium_save_factor":0},
            "source":"public_metadata","source_timestamp":1_700_000_100,"source_version":"13.0.1",
            "current":false,"acknowledged_current":false}
    });
    std::fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "complete", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Verified combat report candidate"));
    assert!(stdout.contains("Machine-readable result"));
    assert!(stdout.contains("\"public_metadata\""));
    assert!(stdout.contains("\"observed\""));
}
