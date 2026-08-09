/// Test with detailed round-by-round logging
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_with_logging_small_scale() {
    println!("\n=== SMALL SCALE WITH LOGGING: 100 Cruisers vs 1000 LFs ===\n");

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
        simulations: 1,
        enable_downscaling: None,
        enable_round_compositions: Some(true),
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("Expected Ratio:");
    println!("  1 Cruiser (5400 HP) needs ~54 LF hits to die");
    println!("  1 LF (800 HP) dies in 1 Cruiser hit");
    println!("  With 6x RF, 1 Cruiser kills ~6 LFs per round");
    println!("  100 Cruisers should kill ~600 LFs per round");
    println!("  Should take ~2 rounds to kill all 1000 LFs");
    println!("  LFs should damage Cruisers during this time");
    println!();

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);
    let result = &results.results[0];

    let cruiser_losses = result.attacker_losses.get(&206).copied().unwrap_or(0);
    let cruiser_remaining = result.attacker_remaining.get(&206).copied().unwrap_or(0);
    let lf_losses = result.defender_losses.get(&204).copied().unwrap_or(0);
    let lf_remaining = result.defender_remaining.get(&204).copied().unwrap_or(0);

    println!("Results:");
    println!("  Rounds: {}", result.rounds);
    println!(
        "  Cruiser Losses: {} / 100 ({:.1}%)",
        cruiser_losses,
        (f64::from(cruiser_losses) / 100.0) * 100.0
    );
    println!("  Cruisers Remaining: {cruiser_remaining}");
    println!(
        "  LF Losses: {} / 1000 ({:.1}%)",
        lf_losses,
        (f64::from(lf_losses) / 1000.0) * 100.0
    );
    println!("  LFs Remaining: {lf_remaining}");
    println!();

    // Calculate expected damage
    let rounds = result.rounds;
    let expected_lf_kills_per_round = 100 * 6; // 100 cruisers × 6 RF
    let expected_total_lf_kills = expected_lf_kills_per_round * u32::from(rounds);

    println!("Expected vs Actual:");
    println!(
        "  Expected LF kills: ~{}",
        expected_total_lf_kills.min(1000)
    );
    println!("  Actual LF kills: {lf_losses}");
    println!();

    // Calculate LF damage to Cruisers
    // In round 1: 1000 LFs shoot
    // First 100 shots break all Cruiser shields
    // Next 900 shots hit hull (900 × 100 = 90,000 dmg)
    // 90,000 / 100 Cruisers = 900 dmg per Cruiser
    // 900 / 5400 = 16.7% hull damage per Cruiser in round 1

    println!("Expected Cruiser Damage:");
    println!("  Round 1: 1000 LFs × 100 dmg = 100,000 total dmg");
    println!("  First 100 shots break shields");
    println!("  Next 900 shots hit hull = 90,000 hull dmg");
    println!("  90,000 / 100 Cruisers = 900 dmg each");
    println!("  900 / 5400 HP = 16.7% hull damage each");
    println!("  At 30% hull → 70% explosion chance");
    println!();

    if cruiser_losses == 0 {
        println!("  ❌ NO Cruisers died! This suggests:");
        println!("     1. LFs are killed too fast (RF too effective)");
        println!("     2. Damage isn't accumulating properly");
        println!("     3. Explosion chance not working");
        println!("     4. Shields regenerating incorrectly");
    } else {
        println!(
            "  ✅ Cruisers ARE dying! Loss rate: {:.1}%",
            (f64::from(cruiser_losses) / 100.0) * 100.0
        );
    }
}
