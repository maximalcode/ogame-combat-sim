/// Direct test of explosion mechanics
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_explosion_mechanics_directly() {
    println!("\n=== EXPLOSION MECHANICS TEST ===\n");

    // Test scenario: Many weak attackers vs few strong defenders
    // This should cause defenders to take gradual hull damage and explode

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    // 5000 Light Fighters (weak, 100 damage each)
    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(204, 5000);

    // 100 Cruisers (strong, 5400 HP each, 100 shield)
    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(206, 100);

    let request = CombatRequest {
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
        use_rapid_fire: false, // Disable RF to keep it simple
        simulations: 10,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("Setup: 5000 LFs vs 100 Cruisers (NO RAPID FIRE)");
    println!("  LF: 100 damage per shot");
    println!("  Cruiser: 100 shield, 5400 HP");
    println!();
    println!("Expected:");
    println!("  Round 1: 5000 LFs shoot");
    println!("    - First 100 shots break shields");
    println!("    - Next 4900 shots hit hull = 490,000 damage");
    println!("    - 490,000 / 100 = 4,900 damage per Cruiser");
    println!("    - 4900 / 5400 = 90.7% hull damage");
    println!("    - Cruisers at ~9% hull → 91% explosion chance!");
    println!("  Most Cruisers should explode in Round 1!");
    println!();

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    let mut total_cruiser_losses = 0u64;
    let mut total_lf_losses = 0u64;

    for result in &results.results {
        total_lf_losses += u64::from(result.attacker_losses.get(&204).copied().unwrap_or(0));
        total_cruiser_losses += u64::from(result.defender_losses.get(&206).copied().unwrap_or(0));
    }

    let avg_lf_losses = total_lf_losses / u64::from(results.simulations);
    let avg_cruiser_losses = total_cruiser_losses / u64::from(results.simulations);

    println!("Results:");
    println!("  Average LF Losses: {avg_lf_losses} / 5000");
    println!("  Average Cruiser Losses: {avg_cruiser_losses} / 100");
    println!("  Average Rounds: {:.1}", results.average_rounds);
    println!("  Attacker (LF) Wins: {}", results.attacker_wins);
    println!("  Defender (Cruiser) Wins: {}", results.defender_wins);
    println!();

    if avg_cruiser_losses > 50 {
        println!("  ✅ EXPLOSION MECHANICS WORKING! Most Cruisers died!");
    } else if avg_cruiser_losses > 10 {
        println!("  ⚠️  Some Cruisers died, but fewer than expected");
    } else {
        println!("  ❌ EXPLOSION NOT WORKING! Almost no Cruisers died!");
        println!("     This confirms explosion mechanic is broken!");
    }
}
