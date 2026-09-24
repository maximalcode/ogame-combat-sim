use combat_ogame_api::reports::{CompletionInput, ReportId, parse_report};
use serde_json::{Value, json};
use std::process::Command;

#[path = "../../combat-ogame-api/tests/support/comparison_input.rs"]
#[allow(dead_code)]
mod support;

#[test]
fn completed_espionage_scenario_runs_through_cli_without_a_comparison() {
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"event_timestamp":1_700_000_000,"failed_ships":false,"failed_defense":false,"failed_research":false},
        "details":{"ships":[{"ship_type":204,"count":12}],"defense":[],"research":[{"research_type":109,"level":10},{"research_type":110,"level":10},{"research_type":111,"level":10}]}
    }});
    let mut evidence = support::evidence();
    evidence.participants.get_mut("A1").unwrap().entities = Some([(204, 20)].into());
    evidence.participants.get_mut("D1").unwrap().technology = None;
    let artifact = CompletionInput {
        candidate: parse_report(&id, &payload.to_string()).unwrap(),
        evidence,
        universe: support::universe(),
    };
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
