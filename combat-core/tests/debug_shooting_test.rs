/// Debug test to see what's happening in combat
use combat_core::Simulator;
use combat_types::{CombatRequest, PartyData, Technology};
use std::collections::HashMap;

#[test]
fn test_small_scale_cruiser_vs_lf() {
    println!("\n=== SMALL SCALE: 10 Cruisers vs 100 LFs ===\n");

    let tech = Technology {
        weapon: 10,
        shield: 10,
        armour: 10,
        ..Default::default()
    };

    // Attacker: 10 Cruisers
    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 10);

    // Defender: 100 Light Fighters
    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 100);

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
        simulations: 1,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    };

    println!("Setup: 10 Cruisers (800 weapon, 100 shield, 5400 HP) with 6x RF");
    println!("   vs  100 LFs (100 weapon, 20 shield, 800 HP)");
    println!();

    let simulator = Simulator::new();
    let results = simulator.simulate_multiple(&request);

    let result = &results.results[0];

    println!("Results:");
    println!("  Rounds: {}", result.rounds);
    println!("  Outcome: {:?}", result.outcome);
    println!();

    let cruiser_losses = result.attacker_losses.get(&206).copied().unwrap_or(0);
    let cruiser_remaining = result.attacker_remaining.get(&206).copied().unwrap_or(0);
    let lf_losses = result.defender_losses.get(&204).copied().unwrap_or(0);
    let lf_remaining = result.defender_remaining.get(&204).copied().unwrap_or(0);

    println!("Losses:");
    println!("  Cruiser Losses: {cruiser_losses} / 10");
    println!("  Cruisers Remaining: {cruiser_remaining}");
    println!("  LF Losses: {lf_losses} / 100");
    println!("  LFs Remaining: {lf_remaining}");
    println!();

    // Calculate expected
    println!("Analysis:");
    println!("  With 6x RF, each Cruiser shoots ~6 times");
    println!("  10 Cruisers × 6 shots = ~60 shots");
    println!("  Each shot kills 1 LF (800 dmg vs 800 HP)");
    println!("  Expected: ~60 LFs dead in round 1");
    println!();
    println!("  Remaining ~40 LFs shoot at Cruisers:");
    println!("  First 10 shots break all Cruiser shields");
    println!("  Next 30 shots hit hull (100 dmg each = 3,000 total)");
    println!("  3,000 dmg / 5,400 HP = Not enough to kill even 1 Cruiser");
    println!();

    if cruiser_losses > 0 {
        println!("  ✅ Cruisers ARE taking damage!");
    } else {
        println!("  ❌ NO Cruisers died - might need more LFs or multiple rounds");
    }
}
