// A scenario script: set up a fleet, run it, print the numbers, assert on
// them. Length here is the scenario being explicit, not a function doing
// too much, so clippy::too_many_lines is waived for the file.
#![allow(clippy::too_many_lines)]

/// Test 10 Cruisers vs 200 LFs - Range Analysis
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_10_cruisers_vs_200_lfs_range() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║     RANGE TEST: 10 Cruisers vs 200 Light Fighters         ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 10); // 10 Cruisers

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 200); // 200 LFs

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
        simulations: 1000,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    // Track outcomes
    let mut min_cruiser_survivors = 10;
    let mut max_cruiser_survivors = 0;
    let mut min_lf_survivors = 200;
    let mut max_lf_survivors = 0;

    let mut cruiser_survivor_counts: HashMap<u32, u32> = HashMap::new();
    let mut lf_survivor_counts: HashMap<u32, u32> = HashMap::new();

    let mut round_distribution: HashMap<u8, u32> = HashMap::new();

    for result in &results.results {
        let cruiser_survivors = result.attacker_remaining.get(&206).copied().unwrap_or(0);
        let lf_survivors = result.defender_remaining.get(&204).copied().unwrap_or(0);

        if cruiser_survivors < min_cruiser_survivors {
            min_cruiser_survivors = cruiser_survivors;
        }
        if cruiser_survivors > max_cruiser_survivors {
            max_cruiser_survivors = cruiser_survivors;
        }

        if lf_survivors < min_lf_survivors {
            min_lf_survivors = lf_survivors;
        }
        if lf_survivors > max_lf_survivors {
            max_lf_survivors = lf_survivors;
        }

        *cruiser_survivor_counts
            .entry(cruiser_survivors)
            .or_insert(0) += 1;
        *lf_survivor_counts.entry(lf_survivors).or_insert(0) += 1;
        *round_distribution.entry(result.rounds).or_insert(0) += 1;
    }

    println!("📊 Results (1000 simulations):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("  Outcome Distribution:");
    println!(
        "    • Attacker Wins: {} ({:.1}%)",
        results.attacker_wins,
        (f64::from(results.attacker_wins) / 10.0)
    );
    println!(
        "    • Defender Wins: {} ({:.1}%)",
        results.defender_wins,
        (f64::from(results.defender_wins) / 10.0)
    );
    println!(
        "    • Draws: {} ({:.1}%)",
        results.draws,
        (f64::from(results.draws) / 10.0)
    );
    println!();

    println!("  Cruiser Survivors:");
    println!("    • Best Case: {max_cruiser_survivors} Cruisers survive");
    println!("    • Worst Case: {min_cruiser_survivors} Cruisers survive");
    println!("    • Range: {min_cruiser_survivors}-{max_cruiser_survivors} survivors");
    println!();

    println!("  LF Survivors:");
    println!("    • Best Case: {min_lf_survivors} LFs survive");
    println!("    • Worst Case: {max_lf_survivors} LFs survive");
    println!("    • Range: {min_lf_survivors}-{max_lf_survivors} survivors");
    println!();

    println!("  Round Distribution:");
    let mut sorted_rounds: Vec<_> = round_distribution.iter().collect();
    sorted_rounds.sort_by_key(|(k, _)| **k);
    for (rounds, count) in sorted_rounds {
        let percentage = (f64::from(*count) / 1000.0) * 100.0;
        println!("    Round {rounds}: {count} times ({percentage:.1}%)");
    }
    println!();

    println!("  Average Rounds: {:.2}", results.average_rounds);
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("🔍 Analysis:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if results.attacker_wins > 0 && results.defender_wins > 0 {
        println!("  ✅ Both sides can win - balanced matchup");
    } else if results.attacker_wins == 1000 {
        println!("  ⚠️  Attackers always win - imbalanced");
    } else if results.defender_wins == 1000 {
        println!("  ⚠️  Defenders always win - imbalanced");
    }

    let cruiser_range = max_cruiser_survivors - min_cruiser_survivors;
    let lf_range = max_lf_survivors - min_lf_survivors;

    println!("  • Cruiser survivor variance: {cruiser_range} (spread of outcomes)");
    println!("  • LF survivor variance: {lf_range} (spread of outcomes)");

    if cruiser_range > 5 || lf_range > 50 {
        println!("  ✅ High variance - RNG working, battles unpredictable");
    } else {
        println!("  ⚠️  Low variance - battles too predictable");
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
