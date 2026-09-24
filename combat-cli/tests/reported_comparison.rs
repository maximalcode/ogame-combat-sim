use std::process::Command;

#[test]
fn cli_reports_conditional_stats_and_rejects_ambiguous_units() {
    let path =
        std::env::temp_dir().join(format!("reported-comparison-{}.json", std::process::id()));
    let mut artifact = serde_json::json!({
        "battle":{"attackers":[{"units":{"204":{"count":250,"weapon":100,"shield":100,"hull":400}}}],"defenders":[{"units":{"408":{"count":1,"weapon":1,"shield":10000,"hull":10000}}}],"rapid_fire":false},
        "rapid_fire_basis":"assumed","source":{"archive_report":null,"battle_timestamp":null},
        "observed":{"winner":"attacker","rounds":1,"attacker_remaining":[{"204":250}],"defender_remaining":[{"408":0}]}
    });
    std::fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "compare-stats", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Conditional reported-stat comparison"));
    let result: serde_json::Value =
        serde_json::from_str(text.split("Machine-readable result:\n").nth(1).unwrap()).unwrap();
    assert_eq!(result["run_count"], 50);
    assert_eq!(result["metrics"][0]["occurrence_count"], 50);
    assert!(!text.contains("verified inputs"));
    assert_eq!(result["metrics"][0]["status"], "unremarkable");
    artifact["battle"]["attackers"][0]["units"]["204"]["armour"] = 123.into();
    std::fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "compare-stats", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(&path.to_string_lossy().to_string()));
}
