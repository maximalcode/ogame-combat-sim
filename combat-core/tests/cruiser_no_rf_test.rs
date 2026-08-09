/// Test WITHOUT rapid fire to isolate the issue
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_cruisers_vs_lf_no_rapid_fire() {
    println!("\n=== 10 Cruisers vs 100 LFs (NO RAPID FIRE) ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 10); // 10 Cruisers

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 100); // 100 LFs

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
        use_rapid_fire: false, // ← DISABLED!
        simulations: 5,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    let mut total_cruiser_losses = 0u64;
    let mut total_lf_losses = 0u64;

    for result in &results.results {
        total_cruiser_losses += u64::from(result.attacker_losses.get(&206).copied().unwrap_or(0));
        total_lf_losses += u64::from(result.defender_losses.get(&204).copied().unwrap_or(0));
    }

    let avg_cruiser_losses = total_cruiser_losses / u64::from(results.simulations);
    let avg_lf_losses = total_lf_losses / u64::from(results.simulations);

    println!("NO RAPID FIRE Results:");
    println!("  Average Cruiser Losses: {avg_cruiser_losses} / 10");
    println!("  Average LF Losses: {avg_lf_losses} / 100");
    println!("  Average Rounds: {:.1}", results.average_rounds);
    println!();

    println!("Expected:");
    println!("  Without RF, Cruisers should still dominate (800 dmg vs 100 dmg)");
    println!("  But LFs should be able to kill SOME Cruisers over 6 rounds");
    println!();

    if avg_cruiser_losses > 0 {
        println!("  ✅ Cruisers ARE dying without RF!");
    } else {
        println!("  ❌ NO Cruisers died even without RF - something is very wrong!");
    }
}
