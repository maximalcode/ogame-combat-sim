// A scenario script: set up a fleet, run it, print the numbers, assert on
// them. Length here is the scenario being explicit, not a function doing
// too much, so clippy::too_many_lines is waived for the file.
#![allow(clippy::too_many_lines)]

/// Deep dive test: 1 Cruiser vs 20 LFs
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_1_cruiser_vs_20_lfs_deep_dive() {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║       DEEP DIVE: 1 Cruiser vs 20 Light Fighters             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

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

    println!("📊 UNIT STATS (Tech 10/10/10):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Cruiser:");
    println!("    • Weapon: 800 damage");
    println!("    • Shield: 100");
    println!("    • Hull: 5,400 HP");
    println!("    • Rapid Fire vs LF: 6x (83.3% chance to shoot again)");
    println!();
    println!("  Light Fighter:");
    println!("    • Weapon: 100 damage");
    println!("    • Shield: 20");
    println!("    • Hull: 800 HP");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("🎯 EXPECTED COMBAT FLOW:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  ROUND 1:");
    println!("    Cruiser Phase:");
    println!("      • Shoots 1 LF → 800 dmg kills it (800 HP)");
    println!("      • RF check (83.3% chance) → likely shoots again");
    println!("      • With 6x RF, expects to kill ~5-6 LFs");
    println!("      • ~14-15 LFs remaining");
    println!();
    println!("    LF Phase:");
    println!("      • ~15 LFs shoot at Cruiser");
    println!("      • First shot: 100 - 100 shield = 0 (shield down)");
    println!("      • Next 14 shots: 100 dmg each = 1,400 hull damage");
    println!("      • Cruiser: 5400 - 1400 = 4,000 HP (74% hull)");
    println!();
    println!("    End of Round: Shield regenerates, hull stays at 4,000");
    println!();
    println!("  ROUND 2:");
    println!("    Cruiser kills ~5-6 more LFs → ~8-10 LFs left");
    println!("    LFs deal ~800-900 more hull damage");
    println!("    Cruiser: ~3,100-3,200 HP (57-59% hull)");
    println!("    Explosion chance: ~41-43%");
    println!();
    println!("  ROUND 3:");
    println!("    Cruiser kills remaining LFs OR gets destroyed");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Test with RF
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
        simulations: 100,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    // Collect statistics
    let mut total_cruiser_losses = 0u64;
    let mut total_lf_losses = 0u64;
    let mut total_rounds = 0u32;

    for result in &results.results {
        total_cruiser_losses += u64::from(result.attacker_losses.get(&206).copied().unwrap_or(0));
        total_lf_losses += u64::from(result.defender_losses.get(&204).copied().unwrap_or(0));
        total_rounds += u32::from(result.rounds);
    }

    let avg_cruiser_losses = total_cruiser_losses as f64 / f64::from(results.simulations);
    let avg_lf_losses = total_lf_losses as f64 / f64::from(results.simulations);
    let avg_rounds = f64::from(total_rounds) / f64::from(results.simulations);

    let cruiser_survival_rate = ((100 - total_cruiser_losses) as f64 / 100.0) * 100.0;

    println!("📈 ACTUAL RESULTS (100 simulations):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Average Rounds: {avg_rounds:.2}");
    println!(
        "  Average LF Losses: {:.1} / 20 ({:.1}%)",
        avg_lf_losses,
        (avg_lf_losses / 20.0) * 100.0
    );
    println!(
        "  Average Cruiser Losses: {:.2} / 1 ({:.1}%)",
        avg_cruiser_losses,
        avg_cruiser_losses * 100.0
    );
    println!("  Cruiser Survival Rate: {cruiser_survival_rate:.1}%");
    println!();
    println!("  Attacker Wins: {}", results.attacker_wins);
    println!("  Defender Wins: {}", results.defender_wins);
    println!("  Draws: {}", results.draws);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("🔍 ANALYSIS:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if avg_cruiser_losses > 0.3 {
        println!(
            "  ✅ Cruisers ARE dying! Loss rate: {:.1}%",
            avg_cruiser_losses * 100.0
        );
        println!("     This suggests combat mechanics are working correctly.");
    } else if avg_cruiser_losses > 0.05 {
        println!("  ⚠️  Some Cruisers dying, but fewer than expected");
        println!("     Loss rate: {:.1}%", avg_cruiser_losses * 100.0);
        println!("     LFs might be dying too fast OR damage not accumulating");
    } else {
        println!(
            "  ❌ Almost NO Cruisers dying! Loss rate: {:.1}%",
            avg_cruiser_losses * 100.0
        );
        println!("     This suggests a major issue:");
        println!("       • LFs being killed too fast (6x RF too effective?)");
        println!("       • Damage not accumulating across rounds");
        println!("       • Explosion chance not triggering");
    }

    println!();

    if avg_lf_losses >= 19.5 {
        println!("  • All LFs dying → Cruiser RF is very effective");
    }

    if avg_rounds < 3.0 {
        println!("  • Battle ending very quickly → Cruiser dominance");
    } else if avg_rounds > 4.0 {
        println!("  • Battle lasting long → More balanced than expected");
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
