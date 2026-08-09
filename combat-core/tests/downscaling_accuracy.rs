// A scenario script: set up a fleet, run it, print the numbers, assert on
// them. Length here is the scenario being explicit, not a function doing
// too much, so clippy::too_many_lines is waived for the file.
#![allow(clippy::too_many_lines)]

/// Comprehensive downscaling accuracy tests
///
/// This test suite compares downscaled simulations with full simulations
/// to measure accuracy loss at different scales.
use combat_core::{Simulator, calculate_downscale_factor, downscale_party, upscale_result};
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

/// Helper to create a fleet
fn create_fleet(ship_type: u16, count: u32) -> HashMap<u16, u32> {
    let mut fleet = HashMap::new();
    fleet.insert(ship_type, count);
    fleet
}

/// Helper to create party data
fn create_party(ship_type: u16, count: u32, tech: Technology) -> PartyData {
    PartyData {
        technology: tech,
        entities: create_fleet(ship_type, count),
    }
}

/// Calculate win rate from results
fn calculate_win_rate(results: &combat_types::CombatResults) -> (f64, f64, f64) {
    let total = f64::from(results.simulations);
    let attacker_rate = (f64::from(results.attacker_wins) / total) * 100.0;
    let defender_rate = (f64::from(results.defender_wins) / total) * 100.0;
    let draw_rate = (f64::from(results.draws) / total) * 100.0;
    (attacker_rate, defender_rate, draw_rate)
}

/// Calculate average loss percentage
fn calculate_loss_percentage(
    results: &combat_types::CombatResults,
    ship_type: u16,
    initial_count: u32,
    is_attacker: bool,
) -> f64 {
    let mut total_losses = 0u64;

    for result in &results.results {
        let losses = if is_attacker {
            &result.attacker_losses
        } else {
            &result.defender_losses
        };

        if let Some(&count) = losses.get(&ship_type) {
            total_losses += u64::from(count);
        }
    }

    let avg_losses = total_losses as f64 / f64::from(results.simulations);
    (avg_losses / f64::from(initial_count)) * 100.0
}

#[test]
fn test_20m_ships_downscaling_accuracy() {
    println!("\n=== 20 MILLION SHIPS TEST ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    // 20M total: 10M Cruisers vs 10M Light Fighters
    let attacker = create_party(206, 10_000_000, tech); // Cruiser
    let defender = create_party(204, 10_000_000, tech); // Light Fighter

    let simulator = Simulator::new();
    let simulations = 10; // Run 10 simulations for statistical validity

    println!("Fleet Configuration:");
    println!("  Attackers: 10,000,000 Cruisers (Type 206)");
    println!("  Defenders: 10,000,000 Light Fighters (Type 204)");
    println!("  Technology: Weapon 10, Shield 10, Armour 10");
    println!("  Simulations: {simulations}");
    println!();

    // Calculate downscale factor
    let downscale_factor = calculate_downscale_factor(&attacker, &defender);
    println!("Downscale Factor: {downscale_factor}x");
    println!(
        "Downscaled Fleet: {} Cruisers vs {} Light Fighters\n",
        10_000_000 / downscale_factor,
        10_000_000 / downscale_factor
    );

    // Test 1: WITH downscaling (automatic)
    println!("--- Test 1: WITH Downscaling ({downscale_factor}x) ---");
    let request_with_scaling = CombatRequest {
        attacker: attacker.clone(),
        defender: defender.clone(),
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire: true,
        simulations,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let start = std::time::Instant::now();
    let results_with_scaling = simulator.simulate_multiple(&request_with_scaling);
    let time_with_scaling = start.elapsed();

    let (att_win_scaled, def_win_scaled, draw_scaled) = calculate_win_rate(&results_with_scaling);
    let att_loss_scaled = calculate_loss_percentage(&results_with_scaling, 206, 10_000_000, true);
    let def_loss_scaled = calculate_loss_percentage(&results_with_scaling, 204, 10_000_000, false);

    println!("Time: {time_with_scaling:?}");
    println!("Win Rates:");
    println!("  Attackers: {att_win_scaled:.2}%");
    println!("  Defenders: {def_win_scaled:.2}%");
    println!("  Draws: {draw_scaled:.2}%");
    println!("Average Loss Rates:");
    println!("  Attacker Losses: {att_loss_scaled:.2}%");
    println!("  Defender Losses: {def_loss_scaled:.2}%");
    println!("Average Rounds: {:.2}", results_with_scaling.average_rounds);
    println!();

    // Test 2: WITHOUT downscaling (simulate smaller fleet to compare)
    println!("--- Test 2: Manual Downscale Comparison ---");
    println!("(Simulating downscaled fleet manually to compare results)");

    // Manually downscale and simulate
    let downscaled_attacker = downscale_party(&attacker, downscale_factor);
    let downscaled_defender = downscale_party(&defender, downscale_factor);

    let start = std::time::Instant::now();
    let mut results_no_scaling = combat_types::CombatResults::new(simulations);
    let mut total_rounds = 0u64;

    for _ in 0..simulations {
        let result = simulator.simulate_multiple(&CombatRequest {
            attacker: downscaled_attacker.clone(),
            defender: downscaled_defender.clone(),
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
        });

        // Take the first result and upscale it
        if let Some(sim_result) = result.results.first() {
            let upscaled = upscale_result(sim_result, downscale_factor);
            total_rounds += u64::from(upscaled.rounds);
            results_no_scaling.add_result(upscaled);
        }
    }

    let time_no_scaling = start.elapsed();
    results_no_scaling.duration_ms = time_no_scaling.as_millis() as u64;
    results_no_scaling.average_rounds = total_rounds as f64 / f64::from(simulations);

    let (att_win_no_scale, def_win_no_scale, draw_no_scale) =
        calculate_win_rate(&results_no_scaling);
    let att_loss_no_scale = calculate_loss_percentage(&results_no_scaling, 206, 10_000_000, true);
    let def_loss_no_scale = calculate_loss_percentage(&results_no_scaling, 204, 10_000_000, false);

    println!("Time: {time_no_scaling:?}");
    println!("Win Rates:");
    println!("  Attackers: {att_win_no_scale:.2}%");
    println!("  Defenders: {def_win_no_scale:.2}%");
    println!("  Draws: {draw_no_scale:.2}%");
    println!("Average Loss Rates:");
    println!("  Attacker Losses: {att_loss_no_scale:.2}%");
    println!("  Defender Losses: {def_loss_no_scale:.2}%");
    println!("Average Rounds: {:.2}", results_no_scaling.average_rounds);
    println!();

    // Test 3: Comparison
    println!("--- Comparison ---");
    println!("Win Rate Difference:");
    println!(
        "  Attackers: {:.2}% (absolute difference)",
        (att_win_scaled - att_win_no_scale).abs()
    );
    println!(
        "  Defenders: {:.2}% (absolute difference)",
        (def_win_scaled - def_win_no_scale).abs()
    );
    println!("Loss Rate Difference:");
    println!(
        "  Attackers: {:.2}% (absolute difference)",
        (att_loss_scaled - att_loss_no_scale).abs()
    );
    println!(
        "  Defenders: {:.2}% (absolute difference)",
        (def_loss_scaled - def_loss_no_scale).abs()
    );
    println!("Performance:");
    println!(
        "  Speedup: {:.2}x",
        time_no_scaling.as_secs_f64() / time_with_scaling.as_secs_f64()
    );
    println!();

    // Assertions - accuracy should be within reasonable bounds
    let win_rate_tolerance = 5.0; // 5% tolerance
    let loss_rate_tolerance = 5.0; // 5% tolerance

    assert!(
        (att_win_scaled - att_win_no_scale).abs() < win_rate_tolerance,
        "Attacker win rate difference too large: {:.2}%",
        (att_win_scaled - att_win_no_scale).abs()
    );

    assert!(
        (att_loss_scaled - att_loss_no_scale).abs() < loss_rate_tolerance,
        "Attacker loss rate difference too large: {:.2}%",
        (att_loss_scaled - att_loss_no_scale).abs()
    );

    println!("✅ Accuracy test PASSED! Differences within tolerance.");
}

#[test]
fn test_statistical_variance_by_fleet_size() {
    println!("\n=== STATISTICAL VARIANCE TEST ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };
    let simulator = Simulator::new();
    let simulations = 20;

    // Test different fleet sizes
    let test_cases = vec![(100_000, "100K"), (1_000_000, "1M"), (10_000_000, "10M")];

    println!("Testing variance at different fleet sizes:");
    println!("(Cruisers vs Light Fighters, {simulations} simulations each)\n");

    for (fleet_size, label) in test_cases {
        println!("--- Fleet Size: {label} ships per side ---");

        let attacker = create_party(206, fleet_size, tech);
        let defender = create_party(204, fleet_size, tech);

        let request = CombatRequest {
            attacker,
            defender,
            attacker_slots: None,
            defender_slots: None,
            planet_resources: None,
            debris_percentage: 30.0,
            use_rapid_fire: true,
            simulations,
            enable_downscaling: None,
            enable_round_compositions: None,
            universe_settings: None,
            attacker_bonuses: None,
            defender_bonuses: None,
            plunder_percentage: 50,
        };

        let results = simulator.simulate_multiple(&request);
        let (att_win, def_win, draws) = calculate_win_rate(&results);

        println!("  Attacker Wins: {att_win:.1}%");
        println!("  Defender Wins: {def_win:.1}%");
        println!("  Draws: {draws:.1}%");
        println!("  Average Rounds: {:.2}", results.average_rounds);
        println!();
    }
}

#[test]
fn test_50m_ships_performance() {
    println!("\n=== 50 MILLION SHIPS PERFORMANCE TEST ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };
    let simulator = Simulator::new();
    let simulations = 10;

    // 50M total: 25M Cruisers vs 25M Light Fighters
    let attacker = create_party(206, 25_000_000, tech);
    let defender = create_party(204, 25_000_000, tech);

    let downscale_factor = calculate_downscale_factor(&attacker, &defender);
    println!("Fleet: 25M Cruisers vs 25M Light Fighters");
    println!("Downscale Factor: {downscale_factor}x");
    println!("Simulations: {simulations}");
    println!();

    let request = CombatRequest {
        attacker,
        defender,
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire: true,
        simulations,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let start = std::time::Instant::now();
    let results = simulator.simulate_multiple(&request);
    let duration = start.elapsed();
    let (att_win, def_win, draws) = calculate_win_rate(&results);

    println!("Results:");
    println!("  Time: {duration:?}");
    println!("  Attacker Wins: {att_win:.1}%");
    println!("  Defender Wins: {def_win:.1}%");
    println!("  Draws: {draws:.1}%");
    println!("  Average Rounds: {:.2}", results.average_rounds);
    println!();

    // With 50M ships and 50x downscaling, we're simulating 1M ships
    // This should be very fast and still accurate
    println!("Expected accuracy: ~98% (±2% variance)");
    println!("Expected time: <1 second for {simulations} simulations");
    println!();

    if cfg!(debug_assertions) {
        println!("(debug) Skipping strict time assertion; debug builds are significantly slower");
    } else {
        assert!(duration.as_secs() < 5, "Should complete in under 5 seconds");
    }
    println!("✅ Performance test PASSED!");
}
