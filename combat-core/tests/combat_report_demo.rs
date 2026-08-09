// A scenario script: set up a fleet, run it, print the numbers, assert on
// them. Length here is the scenario being explicit, not a function doing
// too much, so clippy::too_many_lines is waived for the file.
#![allow(clippy::too_many_lines)]

/// Demonstration of comprehensive combat report generation
use combat_core::{ReportBuilder, Simulator};
use combat_types::{CombatRequest, PartyData, PlanetResources, Technology};
use std::collections::HashMap;

#[test]
fn test_generate_detailed_combat_report() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         COMPREHENSIVE COMBAT REPORT DEMONSTRATION            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // Setup battle: 100 Cruisers vs 1000 Light Fighters
    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 100); // 100 Cruisers

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 1000); // 1000 Light Fighters

    // Defender has resources to plunder
    let planet_resources = Some(PlanetResources {
        metal: 500_000,
        crystal: 300_000,
        deuterium: 100_000,
    });

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
        planet_resources,
        debris_percentage: 30.0,
        use_rapid_fire: true,
        simulations: 100,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    // Run simulation
    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    println!("📊 Simulation Complete:");
    println!("   • {} simulations run", results.simulations);
    println!("   • Duration: {} ms", results.duration_ms);
    println!("   • Average rounds: {:.1}", results.average_rounds);
    println!(
        "   • Attacker wins: {}%",
        (results.attacker_win_rate() * 100.0) as u32
    );
    println!(
        "   • Defender wins: {}%",
        (results.defender_win_rate() * 100.0) as u32
    );
    println!();

    // Generate comprehensive report from first result
    let report_builder = ReportBuilder::new();
    let report = report_builder.build_report(&request, &results.results[0], None, None);

    println!("═══════════════════════════════════════════════════════════════");
    println!("              COMBAT REPORT #{}", report.battle_id);
    println!("═══════════════════════════════════════════════════════════════\n");

    // Battle metadata
    println!("🕐 Timestamp: {}", report.timestamp);
    println!("⚔️  Battle Type: {:?}", report.battle_type);
    println!("🔄 Rounds: {}", report.rounds);
    println!("🏆 Outcome: {:?}", report.outcome);
    println!();

    // Participants
    println!("┌─ ATTACKER ────────────────────────────────────────────────┐");
    println!("│ Name: {}", report.attacker.name);
    println!(
        "│ Tech: Weapon {}, Shield {}, Armor {}",
        report.attacker.technology.weapon,
        report.attacker.technology.shield,
        report.attacker.technology.armour
    );
    println!("└───────────────────────────────────────────────────────────┘\n");

    println!("┌─ DEFENDER ────────────────────────────────────────────────┐");
    println!("│ Name: {}", report.defender.name);
    println!(
        "│ Tech: Weapon {}, Shield {}, Armor {}",
        report.defender.technology.weapon,
        report.defender.technology.shield,
        report.defender.technology.armour
    );
    println!("└───────────────────────────────────────────────────────────┘\n");

    // Fleet states
    println!("┌─ INITIAL FLEETS ──────────────────────────────────────────┐");
    println!("│ Attacker:");
    for (entity_type, count) in &report.attacker_fleet_start.ships {
        println!("│   • Type {entity_type}: {count} ships");
    }
    println!(
        "│   Total Value: {} M / {} C / {} D",
        report.attacker_fleet_start.total_value.metal,
        report.attacker_fleet_start.total_value.crystal,
        report.attacker_fleet_start.total_value.deuterium
    );
    println!("│");
    println!("│ Defender:");
    for (entity_type, count) in &report.defender_fleet_start.ships {
        println!("│   • Type {entity_type}: {count} ships");
    }
    println!(
        "│   Total Value: {} M / {} C / {} D",
        report.defender_fleet_start.total_value.metal,
        report.defender_fleet_start.total_value.crystal,
        report.defender_fleet_start.total_value.deuterium
    );
    println!("└───────────────────────────────────────────────────────────┘\n");

    // Losses
    println!("┌─ LOSSES ──────────────────────────────────────────────────┐");
    println!("│ Attacker Lost:");
    for (entity_type, count) in &report.attacker_losses.ships {
        println!("│   • Type {entity_type}: {count} ships");
    }
    println!(
        "│   Cost: {} M / {} C / {} D",
        report.economics.attacker_losses_cost.metal,
        report.economics.attacker_losses_cost.crystal,
        report.economics.attacker_losses_cost.deuterium
    );
    println!("│");
    println!("│ Defender Lost:");
    for (entity_type, count) in &report.defender_losses.ships {
        println!("│   • Type {entity_type}: {count} ships");
    }
    println!(
        "│   Cost: {} M / {} C / {} D",
        report.economics.defender_losses_cost.metal,
        report.economics.defender_losses_cost.crystal,
        report.economics.defender_losses_cost.deuterium
    );
    println!("└───────────────────────────────────────────────────────────┘\n");

    // Economics
    println!("┌─ ECONOMIC SUMMARY ────────────────────────────────────────┐");
    println!("│ 💎 Debris Field:");
    println!("│   • Metal: {}", report.economics.debris_field.metal);
    println!("│   • Crystal: {}", report.economics.debris_field.crystal);
    println!("│   • Total: {}", report.economics.debris_field.total());
    println!("│");
    println!("│ 🌙 Moon Chance: {:.1}%", report.economics.moon_chance);
    println!("│");

    if let Some(harvest) = &report.economics.harvest_info {
        println!("│ ♻️  Harvest Info:");
        println!("│   • Recyclers Needed: {}", harvest.recyclers_needed);
        println!(
            "│   • Estimated Time: {} seconds",
            harvest.harvest_time_seconds
        );
        println!("│");
    }

    println!("│ 💰 Plunder:");
    println!("│   • Metal: {}", report.economics.plunder.metal);
    println!("│   • Crystal: {}", report.economics.plunder.crystal);
    println!("│   • Deuterium: {}", report.economics.plunder.deuterium);
    println!("│   • Total: {}", report.economics.plunder.total());
    println!("│");
    println!("│ 📊 Profit/Loss:");
    println!("│   • Attacker: {}", report.economics.attacker_profit);
    println!("│   • Defender: {}", report.economics.defender_profit);
    println!("└───────────────────────────────────────────────────────────┘\n");

    // Survivors
    println!("┌─ SURVIVORS ───────────────────────────────────────────────┐");
    println!("│ Attacker:");
    if report.attacker_fleet_end.ships.is_empty() {
        println!("│   • All destroyed!");
    } else {
        for (entity_type, count) in &report.attacker_fleet_end.ships {
            println!("│   • Type {entity_type}: {count} ships");
        }
    }
    println!("│");
    println!("│ Defender:");
    if report.defender_fleet_end.ships.is_empty() {
        println!("│   • All destroyed!");
    } else {
        for (entity_type, count) in &report.defender_fleet_end.ships {
            println!("│   • Type {entity_type}: {count} ships");
        }
    }
    println!("└───────────────────────────────────────────────────────────┘\n");

    // Generate JSON report for API
    let json_report = serde_json::to_string_pretty(&report).unwrap();
    println!("📄 JSON Export (first 500 chars):");
    println!("{}", &json_report[..json_report.len().min(500)]);
    println!("...\n");

    // Generate summary report from all simulations
    let summary_report = report_builder.build_summary_report(&request, &results);

    println!("═══════════════════════════════════════════════════════════════");
    println!(
        "         AGGREGATED SUMMARY ({} Simulations)",
        results.simulations
    );
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("📊 Average Results:");
    println!("   • Rounds: {}", summary_report.rounds);
    println!(
        "   • Attacker Losses: {} / 100 Cruisers ({:.0}%)",
        summary_report
            .attacker_losses
            .ships
            .get(&206)
            .copied()
            .unwrap_or(0),
        f64::from(
            summary_report
                .attacker_losses
                .ships
                .get(&206)
                .copied()
                .unwrap_or(0)
        )
    );
    println!(
        "   • Defender Losses: {} / 1000 LFs ({:.0}%)",
        summary_report
            .defender_losses
            .ships
            .get(&204)
            .copied()
            .unwrap_or(0),
        f64::from(
            summary_report
                .defender_losses
                .ships
                .get(&204)
                .copied()
                .unwrap_or(0)
        ) / 10.0
    );
    println!(
        "   • Avg Debris: {}",
        summary_report.economics.debris_field.total()
    );
    println!(
        "   • Avg Plunder: {}",
        summary_report.economics.plunder.total()
    );
    println!(
        "   • Avg Attacker Profit: {}",
        summary_report.economics.attacker_profit
    );
    println!(
        "   • Avg Defender Profit: {}",
        summary_report.economics.defender_profit
    );
    println!();

    println!("✅ Combat report generation successful!");
    println!("   Report ID: {}", report.battle_id);
    println!("   Summary ID: {}", summary_report.battle_id);
}

#[test]
fn test_json_serialization() {
    // Quick test to ensure reports serialize properly to JSON
    let tech = Technology {
        weapon: 5,
        shield: 5,
        armour: 5,
        ..Default::default()
    };

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(204, 10); // 10 Light Fighters

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(401, 50); // 50 Rocket Launchers

    let request = CombatRequest {
        attacker: PartyData {
            technology: tech,
            entities: attacker_fleet,
        },
        defender: PartyData {
            technology: tech,
            entities: defender_fleet,
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
    let results = simulator.simulate_multiple(&request);

    let report_builder = ReportBuilder::new();
    let report = report_builder.build_report(&request, &results.results[0], None, None);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&report).expect("Failed to serialize report");

    println!("\n📄 Full JSON Report:\n{json}");

    // Deserialize back
    let _deserialized: combat_types::CombatReport =
        serde_json::from_str(&json).expect("Failed to deserialize report");

    println!("\n✅ JSON serialization/deserialization successful!");
}
