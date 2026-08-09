/// Minimal test to debug rapid fire logic
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_minimal_rf_debug() {
    println!("\n=== MINIMAL RF DEBUG: 1 Cruiser vs 10 LFs ===\n");

    let tech = Technology::default(); // No tech to keep simple

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 1); // 1 Cruiser (400 weapon, 50 shield, 2700 HP)

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 10); // 10 LFs (50 weapon, 10 shield, 400 HP)

    println!("Setup:");
    println!("  1 Cruiser: 400 weapon, 50 shield, 2700 HP, 6x RF vs LF");
    println!("  10 LFs: 50 weapon, 10 shield, 400 HP");
    println!();

    // Test with RF
    let request_rf = CombatRequest {
        attacker: PartyData {
            technology: tech,
            entities: attacker_fleet.clone(),
        },
        defender: PartyData {
            technology: tech,
            entities: defender_fleet.clone(),
        },
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire: true,
        simulations: 1,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let simulator = Simulator::new();
    let results_rf = simulator.simulate_multiple(&request_rf);
    let result_rf = &results_rf.results[0];

    let lf_losses_rf = result_rf.defender_losses.get(&204).copied().unwrap_or(0);
    let cruiser_losses_rf = result_rf.attacker_losses.get(&206).copied().unwrap_or(0);

    println!("WITH RAPID FIRE:");
    println!("  LFs killed: {lf_losses_rf} / 10");
    println!("  Cruiser killed: {cruiser_losses_rf} / 1");
    println!("  Rounds: {}", result_rf.rounds);
    println!();

    // Test without RF
    let request_no_rf = CombatRequest {
        attacker: PartyData {
            technology: tech,
            entities: attacker_fleet.clone(),
        },
        defender: PartyData {
            technology: tech,
            entities: defender_fleet.clone(),
        },
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire: false,
        simulations: 1,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let results_no_rf = simulator.simulate_multiple(&request_no_rf);
    let result_no_rf = &results_no_rf.results[0];

    let lf_losses_no_rf = result_no_rf.defender_losses.get(&204).copied().unwrap_or(0);
    let cruiser_losses_no_rf = result_no_rf.attacker_losses.get(&206).copied().unwrap_or(0);

    println!("WITHOUT RAPID FIRE:");
    println!("  LFs killed: {lf_losses_no_rf} / 10");
    println!("  Cruiser killed: {cruiser_losses_no_rf} / 1");
    println!("  Rounds: {}", result_no_rf.rounds);
    println!();

    println!("Analysis:");
    println!("  With 6x RF, Cruiser should kill ~6 LFs in round 1");
    println!("  Without RF, Cruiser kills only 1 LF per round");
    println!();

    if lf_losses_rf > lf_losses_no_rf {
        println!("  ✅ RF IS working (kills more LFs)");
        println!("     RF: {lf_losses_rf} LFs killed");
        println!("     No RF: {lf_losses_no_rf} LFs killed");
    } else {
        println!("  ❌ RF NOT working (same or fewer LFs killed)");
    }
}
