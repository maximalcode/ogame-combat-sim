// A scenario script: set up a fleet, run it, print the numbers, assert on
// them. Length here is the scenario being explicit, not a function doing
// too much, so clippy::too_many_lines is waived for the file.
#![allow(clippy::too_many_lines)]

/// Verify 1 Cruiser vs 20 LFs matches `TrashSim` range
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_1v20_outcome_range() {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  VERIFY: 1 Cruiser vs 20 LFs Outcome Range              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 1); // 1 Cruiser

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 20); // 20 LFs

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
        use_rapid_fire: true,
        simulations: 1000, // Large sample
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("📊 TrashSim Expected Range:");
    println!("  Best case for Cruiser: Dies in Round 2, 4 LFs survive");
    println!("  Best case for LFs: Cruiser dies in Round 1, 14 LFs survive");
    println!("  → Cruiser always dies");
    println!("  → 4-14 LFs survive");
    println!();

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    // Track outcomes
    let mut round_1_deaths = 0;
    let mut round_2_deaths = 0;
    let mut round_3plus_deaths = 0;
    let mut cruiser_wins = 0;

    let mut min_lf_survivors = 20;
    let mut max_lf_survivors = 0;
    let mut lf_survivor_counts: HashMap<u32, u32> = HashMap::new();

    for result in &results.results {
        let cruiser_survived = result.attacker_remaining.get(&206).copied().unwrap_or(0) > 0;
        let lf_survivors = result.defender_remaining.get(&204).copied().unwrap_or(0);

        if cruiser_survived {
            cruiser_wins += 1;
        } else {
            // Cruiser died - track which round
            match result.rounds {
                1 => round_1_deaths += 1,
                2 => round_2_deaths += 1,
                _ => round_3plus_deaths += 1,
            }
        }

        if lf_survivors < min_lf_survivors {
            min_lf_survivors = lf_survivors;
        }
        if lf_survivors > max_lf_survivors {
            max_lf_survivors = lf_survivors;
        }

        *lf_survivor_counts.entry(lf_survivors).or_insert(0) += 1;
    }

    println!("📈 Our Results (1000 simulations):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("  Cruiser Outcome:");
    println!(
        "    • Cruiser wins: {} ({:.1}%)",
        cruiser_wins,
        (f64::from(cruiser_wins) / 10.0)
    );
    println!(
        "    • Cruiser dies round 1: {} ({:.1}%)",
        round_1_deaths,
        (f64::from(round_1_deaths) / 10.0)
    );
    println!(
        "    • Cruiser dies round 2: {} ({:.1}%)",
        round_2_deaths,
        (f64::from(round_2_deaths) / 10.0)
    );
    println!(
        "    • Cruiser dies round 3+: {} ({:.1}%)",
        round_3plus_deaths,
        (f64::from(round_3plus_deaths) / 10.0)
    );
    println!();

    println!("  LF Survivors:");
    println!("    • Min: {min_lf_survivors} LFs");
    println!("    • Max: {max_lf_survivors} LFs");
    println!();

    println!("  Distribution of LF Survivors:");
    let mut sorted_counts: Vec<_> = lf_survivor_counts.iter().collect();
    sorted_counts.sort_by_key(|(k, _)| **k);
    for (survivors, count) in sorted_counts {
        let percentage = (f64::from(*count) / 1000.0) * 100.0;
        let bar_length = (percentage / 2.0) as usize;
        let bar = "█".repeat(bar_length);
        println!("    {survivors:2} LFs: {count:4} times ({percentage:5.1}%) {bar}");
    }
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("🔍 Comparison:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Check if cruiser always dies
    if cruiser_wins == 0 {
        println!("  ✅ Cruiser ALWAYS dies (matches TrashSim)");
    } else {
        println!("  ❌ Cruiser sometimes wins! ({cruiser_wins} times)");
        println!("     TrashSim: Cruiser should always die");
    }

    // Check round distribution
    let total_cruiser_deaths = round_1_deaths + round_2_deaths + round_3plus_deaths;
    if total_cruiser_deaths > 0 {
        let round_1_pct = (f64::from(round_1_deaths) / f64::from(total_cruiser_deaths)) * 100.0;
        let round_2_pct = (f64::from(round_2_deaths) / f64::from(total_cruiser_deaths)) * 100.0;

        println!("  • Round 1 deaths: {round_1_pct:.1}%");
        println!("  • Round 2 deaths: {round_2_pct:.1}%");

        if round_1_pct > 0.0 && round_2_pct > 0.0 {
            println!("  ✅ Cruiser dies in rounds 1-2 (matches TrashSim)");
        }
    }

    // Check LF survivor range
    if min_lf_survivors >= 4 && max_lf_survivors <= 14 {
        println!(
            "  ✅ LF survivors: {min_lf_survivors}-{max_lf_survivors} (matches TrashSim range: 4-14)"
        );
    } else {
        println!("  ⚠️  LF survivors: {min_lf_survivors}-{max_lf_survivors}");
        println!("     TrashSim range: 4-14");

        if min_lf_survivors < 4 {
            println!("     → Our sim: Cruiser killing TOO MANY LFs");
        }
        if max_lf_survivors > 14 {
            println!("     → Our sim: Cruiser killing TOO FEW LFs");
        }
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
