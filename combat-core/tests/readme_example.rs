use combat_core::{ReportBuilder, Simulator};
use combat_types::CombatRequest;

#[test]
fn readme_library_example_compiles_and_runs() -> Result<(), Box<dyn std::error::Error>> {
    let request: CombatRequest = serde_json::from_str(
        r#"{
    "attacker": { "technology": {"weapon": 10, "shield": 10, "armour": 10},
                  "entities": {"204": 100} },
    "defender": { "technology": {"weapon": 8, "shield": 8, "armour": 8},
                  "entities": {"401": 50} },
    "use_rapid_fire": true,
    "simulations": 1000
}"#,
    )?;

    let results = Simulator::new().simulate_multiple(&request);
    println!("attacker wins {:.1}%", results.attacker_win_rate() * 100.0);

    let report = ReportBuilder::new().build_summary_report(&request, &results);
    assert_eq!(results.simulations, 1000);
    assert!(report.rounds >= 1);
    Ok(())
}
