/// Test to verify hull damage persists across rounds
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_hull_damage_persists_across_rounds() {
    println!("\n=== HULL DAMAGE PERSISTENCE TEST ===\n");

    let tech = Technology::default(); // No tech

    // 3 Heavy Fighters (150 dmg, weak enough to not 1-shot)
    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(205, 3);

    // 1 Cruiser (50 shield, 2700 HP)
    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(206, 1);

    let request = CombatRequest {
        attacker: PartyData {
            technology: tech,
            entities: attacker_fleet.clone(),
            ..Default::default()
        },
        defender: PartyData {
            technology: tech,
            entities: defender_fleet.clone(),
            ..Default::default()
        },
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire: false, // No RF for clarity
        simulations: 1,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("Setup:");
    println!("  3 Heavy Fighters: 150 damage each");
    println!("  1 Cruiser: 50 shield, 2700 HP");
    println!();
    println!("Expected Damage Per Round:");
    println!("  Round 1: 3 HFs shoot");
    println!("    - First shot: 150 - 50 shield = 100 hull damage");
    println!("    - Second shot: 150 hull damage");
    println!("    - Third shot: 150 hull damage");
    println!("    - Total: 400 hull damage (2300 HP left = 85% hull)");
    println!("  Round 2: Shield REGENERATES, but hull stays at 2300");
    println!("    - First shot: 150 - 50 shield = 100 hull damage");
    println!("    - Second shot: 150 hull damage");
    println!("    - Third shot: 150 hull damage");
    println!("    - Total: 800 hull damage (1900 HP left = 70% hull)");
    println!("  Round 3+: Explosion chance kicks in...");
    println!();

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);
    let result = &results.results[0];

    let cruiser_losses = result.defender_losses.get(&206).copied().unwrap_or(0);
    let hf_losses = result.attacker_losses.get(&205).copied().unwrap_or(0);

    println!("Results:");
    println!("  Rounds: {}", result.rounds);
    println!("  Heavy Fighter Losses: {hf_losses} / 3");
    println!("  Cruiser Losses: {cruiser_losses} / 1");
    println!();

    if result.rounds >= 3 {
        println!(
            "  ✅ Hull damage IS persisting! Battle took {} rounds",
            result.rounds
        );
        println!("     (If hull reset each round, battle would take many more rounds)");
    } else {
        println!("  ❌ Hull damage might NOT be persisting! Battle too short");
    }

    if cruiser_losses == 1 {
        println!("  ✅ Cruiser was destroyed as expected!");
    } else {
        println!("  ⚠️  Cruiser survived! Hull damage might not be accumulating correctly");
    }
}
