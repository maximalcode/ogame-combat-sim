use std::process::Command;
#[path = "../../combat-ogame-api/tests/support/acs_input.rs"]
mod imported;
// This fixture module also serves library tests that use candidate builders.
#[allow(dead_code)]
#[path = "../../combat-ogame-api/tests/support/comparison_input.rs"]
mod support;

#[test]
fn cli_shows_verified_participants_and_aggregate_fallback() {
    let artifact = imported::artifact([Some(11), Some(11), Some(12), Some(13)]);
    let path = std::env::temp_dir().join(format!("acs-cli-{}.json", std::process::id()));
    std::fs::write(&path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    for command in ["complete", "compare"] {
        let result = Command::new(env!("CARGO_BIN_EXE_combat-cli"))
            .args(["report", command, "--file"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let text = String::from_utf8(result.stdout).unwrap();
        for slot in ["A1", "A2", "D1", "D2"] {
            assert!(text.contains(&format!("Participant {slot}")), "{text}");
        }
        if command == "compare" {
            assert!(text.contains("A1.losses.count: not_assessable"));
            assert!(text.contains("aggregate side totals"));
            assert!(text.contains("attribution"));
        }
        assert!(!text.contains("private-name"));
    }
    std::fs::remove_file(path).unwrap();
}
