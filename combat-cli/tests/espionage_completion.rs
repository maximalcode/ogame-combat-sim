use serde_json::Value;
use std::process::Command;

#[path = "../../combat-ogame-api/tests/support/espionage_input.rs"]
mod espionage_input;

#[test]
fn completed_espionage_scenario_runs_through_cli_without_a_comparison() {
    let artifact = espionage_input::scenario();
    let path = std::env::temp_dir().join(format!("espionage-cli-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let complete = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "complete", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(complete.status.success());
    let output = String::from_utf8(complete.stdout).unwrap();
    assert!(output.contains("Verified espionage scenario"), "{output}");
    assert!(output.contains("snapshot.provenance"));
    let machine: Value =
        serde_json::from_str(output.split("Machine-readable result:\n").nth(1).unwrap()).unwrap();
    assert!(machine["input"]["observed"].is_null());
    let compare = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["report", "compare", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(!compare.status.success());
    assert!(
        String::from_utf8(compare.stderr)
            .unwrap()
            .contains("observed combat report")
    );
    std::fs::write(
        &path,
        serde_json::to_vec(&machine["input"]["request"]).unwrap(),
    )
    .unwrap();
    let simulate = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
        .args(["sim", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(
        simulate.status.success(),
        "{}",
        String::from_utf8_lossy(&simulate.stderr)
    );
}
