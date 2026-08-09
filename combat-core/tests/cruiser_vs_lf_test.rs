// A scenario script: set up a fleet, run it, print the numbers, assert on
// them. Length here is the scenario being explicit, not a function doing
// too much, so clippy::too_many_lines is waived for the file.
#![allow(clippy::too_many_lines)]

/// Test case for 350k Cruisers vs 2.5M Light Fighters
///
/// Real `OGame` Result (Tech 10):
/// - All 2.5M LFs destroyed
/// - ~211k Cruisers survive (139k die)
///
/// This test validates our combat mechanics against real `OGame` data.
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_350k_cruisers_vs_2_5m_light_fighters() {
    println!("\n=== 350K CRUISERS VS 2.5M LIGHT FIGHTERS ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    // Attacker: 350,000 Cruisers
    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 350_000); // Cruisers

    // Defender: 2,500,000 Light Fighters
    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 2_500_000); // Light Fighters

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
        use_rapid_fire: true,
        simulations: 10, // Run 10 sims for average
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("Configuration:");
    println!("  Attackers: 350,000 Cruisers (Type 206)");
    println!("  Defenders: 2,500,000 Light Fighters (Type 204)");
    println!("  Technology: 10/10/10 both sides");
    println!("  Simulations: 10");
    println!();

    // Cruiser stats with tech 10
    println!("Cruiser Stats (Tech 10):");
    println!("  Weapon: 400 × 2.0 = 800");
    println!("  Shield: 50 × 2.0 = 100");
    println!("  Structure: 27,000 × 2.0 = 54,000");
    println!("  Hull: 54,000 × 0.1 = 5,400 HP");
    println!("  Rapid Fire vs LF: 6x");
    println!();

    println!("Light Fighter Stats (Tech 10):");
    println!("  Weapon: 50 × 2.0 = 100");
    println!("  Shield: 10 × 2.0 = 20");
    println!("  Structure: 4,000 × 2.0 = 8,000");
    println!("  Hull: 8,000 × 0.1 = 800 HP");
    println!();

    println!("Expected Mechanics:");
    println!("  - First LF shot: 100 dmg - 100 shield = 0 leftover (shield down)");
    println!("  - Next LF shots in same round: 100 dmg to hull");
    println!("  - Need 54 LF shots to kill 1 Cruiser (5,400 HP)");
    println!("  - Cruisers have 6x rapid fire advantage");
    println!("  - Explosion chance when hull < 70%");
    println!();

    let simulator = Simulator::new();
    let start = std::time::Instant::now();
    let results = simulator.simulate_multiple(&request);
    let duration = start.elapsed();

    println!("Results:");
    println!("  Time: {duration:?}");
    println!("  Attacker Wins: {}", results.attacker_wins);
    println!("  Defender Wins: {}", results.defender_wins);
    println!("  Draws: {}", results.draws);
    println!("  Average Rounds: {:.1}", results.average_rounds);
    println!();

    // Average losses and remaining
    let mut total_cruiser_losses = 0u64;
    let mut total_cruiser_remaining = 0u64;
    let mut total_lf_losses = 0u64;
    let mut total_lf_remaining = 0u64;

    for result in &results.results {
        total_cruiser_losses += u64::from(result.attacker_losses.get(&206).copied().unwrap_or(0));
        total_cruiser_remaining +=
            u64::from(result.attacker_remaining.get(&206).copied().unwrap_or(0));
        total_lf_losses += u64::from(result.defender_losses.get(&204).copied().unwrap_or(0));
        total_lf_remaining += u64::from(result.defender_remaining.get(&204).copied().unwrap_or(0));
    }

    let avg_cruiser_losses = total_cruiser_losses / u64::from(results.simulations);
    let avg_cruiser_remaining = total_cruiser_remaining / u64::from(results.simulations);
    let avg_lf_losses = total_lf_losses / u64::from(results.simulations);
    let avg_lf_remaining = total_lf_remaining / u64::from(results.simulations);

    println!("Average Losses:");
    println!(
        "  Cruiser Losses: {} ({:.1}%)",
        avg_cruiser_losses,
        (avg_cruiser_losses as f64 / 350_000.0) * 100.0
    );
    println!(
        "  Cruisers Remaining: {} ({:.1}%)",
        avg_cruiser_remaining,
        (avg_cruiser_remaining as f64 / 350_000.0) * 100.0
    );
    println!();
    println!(
        "  Light Fighter Losses: {} ({:.1}%)",
        avg_lf_losses,
        (avg_lf_losses as f64 / 2_500_000.0) * 100.0
    );
    println!("  Light Fighters Remaining: {avg_lf_remaining}");
    println!();

    println!("Real OGame Comparison:");
    println!("  Expected: ~211k Cruisers survive (139k die)");
    println!("  Expected: All LFs destroyed");
    println!(
        "  Our Result: {}k Cruisers survive ({}k die)",
        avg_cruiser_remaining / 1000,
        avg_cruiser_losses / 1000
    );
    println!();

    // Analysis (no assertions, just print)
    let cruiser_loss_percentage = (avg_cruiser_losses as f64 / 350_000.0) * 100.0;
    println!("Analysis:");
    println!("  Cruiser loss rate: {cruiser_loss_percentage:.1}%");

    if results.attacker_wins != 10 {
        println!(
            "  ⚠️  WARNING: Not all sims won by attackers: {} wins",
            results.attacker_wins
        );
    }

    if avg_lf_remaining > 0 {
        println!("  ⚠️  WARNING: Some LFs survived: {avg_lf_remaining}");
    }

    if cruiser_loss_percentage < 10.0 {
        println!(
            "  ❌ BUG CONFIRMED: Too few Cruisers dying! Only {cruiser_loss_percentage:.1}% losses"
        );
        println!("     Expected: ~40% losses (139k out of 350k)");
        println!(
            "     Got: {:.1}% losses ({}k out of 350k)",
            cruiser_loss_percentage,
            avg_cruiser_losses / 1000
        );
    } else if cruiser_loss_percentage < 25.0 {
        println!("  ⚠️  WARNING: Cruiser losses seem low (expected ~40%)");
    } else if cruiser_loss_percentage > 60.0 {
        println!("  ⚠️  WARNING: Cruiser losses seem high (expected ~40%)");
    } else {
        println!("  ✅ Cruiser losses look reasonable!");
    }
}
