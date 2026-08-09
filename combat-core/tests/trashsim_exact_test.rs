/// Exact `TrashSim` test case: 100 Cruisers vs 1000 LFs with RF
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_100_cruisers_vs_1000_lfs_with_rf() {
    println!("\n=== TRASHSIM EXACT TEST: 100 Cruisers vs 1000 LFs (WITH RF) ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 100); // 100 Cruisers

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 1000); // 1000 LFs

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
        use_rapid_fire: true, // ← ENABLED
        simulations: 100,     // Run 100 times for good average
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("TrashSim Results:");
    println!("  All 1000 LFs die in 4 rounds");
    println!("  75 Cruisers survive (25 die = 25% loss rate)");
    println!();

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

    println!("Our Results:");
    println!("  Average Rounds: {:.1}", results.average_rounds);
    println!(
        "  LF Losses: {} / 1000 ({:.1}%)",
        avg_lf_losses,
        (avg_lf_losses as f64 / 1000.0) * 100.0
    );
    println!(
        "  Cruiser Losses: {} / 100 ({:.1}%)",
        avg_cruiser_losses,
        (avg_cruiser_losses as f64 / 100.0) * 100.0
    );
    println!("  Cruisers Remaining: {}", 100 - avg_cruiser_losses);
    println!();

    println!("Comparison:");
    println!("  TrashSim: 25 Cruisers die (25%)");
    println!(
        "  Our Sim:  {} Cruisers die ({:.1}%)",
        avg_cruiser_losses,
        (avg_cruiser_losses as f64 / 100.0) * 100.0
    );
    println!();

    if (20..=30).contains(&avg_cruiser_losses) {
        println!("  ✅ PERFECT MATCH! Within expected range!");
    } else if avg_cruiser_losses >= 10 {
        println!("  ⚠️  CLOSE but not exact. Some Cruisers dying.");
    } else {
        println!("  ❌ MISMATCH! Far fewer Cruisers dying than expected.");
        println!("     Expected: ~25 deaths");
        println!("     Got: {avg_cruiser_losses} deaths");
    }
}
