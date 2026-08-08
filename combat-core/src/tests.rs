use crate::simulator::Simulator;
use combat_types::{CombatRequest, FleetComposition, PartyData, Technology};
use std::collections::HashMap;

/// Helper to create a combat request
fn create_request(
    attacker_fleet: FleetComposition,
    attacker_tech: Technology,
    defender_fleet: FleetComposition,
    defender_tech: Technology,
    simulations: u32,
    use_rapid_fire: bool,
) -> CombatRequest {
    CombatRequest {
        attacker: PartyData {
            technology: attacker_tech,
            entities: attacker_fleet,
        },
        defender: PartyData {
            technology: defender_tech,
            entities: defender_fleet,
        },
        attacker_slots: None,
        defender_slots: None,
        planet_resources: None,
        debris_percentage: 30.0,
        use_rapid_fire,
        simulations,
        enable_downscaling: None,
        enable_round_compositions: None,
        universe_settings: None,
        attacker_bonuses: None,
        defender_bonuses: None,
        plunder_percentage: 50,
    }
}

#[test]
fn test_light_fighters_vs_rocket_launchers() {
    let simulator = Simulator::new();

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(204, 50); // 50 Light Fighters

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(401, 25); // 25 Rocket Launchers

    let request = create_request(
        attacker_fleet,
        Technology::default(),
        defender_fleet,
        Technology::default(),
        100,
        true,
    );

    let results = simulator.simulate_multiple(&request);

    // Light fighters should have advantage (higher weapon)
    assert!(results.attacker_win_rate() > 0.5);
    println!("Light Fighters vs Rocket Launchers:");
    println!(
        "  Attacker win rate: {:.1}%",
        results.attacker_win_rate() * 100.0
    );
    println!(
        "  Defender win rate: {:.1}%",
        results.defender_win_rate() * 100.0
    );
    println!("  Draw rate: {:.1}%", results.draw_rate() * 100.0);
}

#[test]
fn test_technology_advantage() {
    let simulator = Simulator::new();

    let mut fleet = HashMap::new();
    fleet.insert(204, 50); // 50 Light Fighters each

    // Test with no tech vs high tech
    let request_no_tech = create_request(
        fleet.clone(),
        Technology::default(),
        fleet.clone(),
        Technology {
            weapon: 10,
            shield: 10,
            armour: 10,
            ..Default::default()
        },
        100,
        true,
    );

    let results = simulator.simulate_multiple(&request_no_tech);

    // Side with tech 10 should dominate
    assert!(results.defender_win_rate() > 0.7);
    println!("Tech 0 vs Tech 10:");
    println!(
        "  No tech win rate: {:.1}%",
        results.attacker_win_rate() * 100.0
    );
    println!(
        "  Tech 10 win rate: {:.1}%",
        results.defender_win_rate() * 100.0
    );
}

#[test]
fn test_cruiser_vs_light_fighter_rapid_fire() {
    let simulator = Simulator::new();

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(206, 10); // 10 Cruisers

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(204, 50); // 50 Light Fighters

    // Test with rapid fire enabled
    let request_rf = create_request(
        attacker_fleet.clone(),
        Technology::default(),
        defender_fleet.clone(),
        Technology::default(),
        100,
        true,
    );

    let results_rf = simulator.simulate_multiple(&request_rf);

    // Test without rapid fire
    let request_no_rf = create_request(
        attacker_fleet,
        Technology::default(),
        defender_fleet,
        Technology::default(),
        100,
        false,
    );

    let results_no_rf = simulator.simulate_multiple(&request_no_rf);

    println!("Cruisers vs Light Fighters (Rapid Fire):");
    println!(
        "  With RF - Attacker wins: {}%",
        (results_rf.attacker_win_rate() * 100.0) as u32
    );
    println!(
        "  Without RF - Attacker wins: {}%",
        (results_no_rf.attacker_win_rate() * 100.0) as u32
    );

    // Cruisers have 6x rapid fire vs Light Fighters, so RF should help significantly
    assert!(results_rf.attacker_win_rate() > results_no_rf.attacker_win_rate());
}

#[test]
fn test_bomber_vs_defenses() {
    let simulator = Simulator::new();

    let mut attacker_fleet = HashMap::new();
    attacker_fleet.insert(211, 5); // 5 Bombers

    let mut defender_fleet = HashMap::new();
    defender_fleet.insert(401, 50); // 50 Rocket Launchers
    defender_fleet.insert(402, 25); // 25 Light Lasers

    let request = create_request(
        attacker_fleet,
        Technology {
            weapon: 10,
            shield: 10,
            armour: 10,
            ..Default::default()
        },
        defender_fleet,
        Technology {
            weapon: 8,
            shield: 8,
            armour: 8,
            ..Default::default()
        },
        100,
        true,
    );

    let results = simulator.simulate_multiple(&request);

    println!("Bombers vs Defenses:");
    println!(
        "  Attacker win rate: {:.1}%",
        results.attacker_win_rate() * 100.0
    );
    println!(
        "  Defender win rate: {:.1}%",
        results.defender_win_rate() * 100.0
    );

    // Bombers have high rapid fire vs defenses, should perform well
    assert!(results.attacker_win_rate() > 0.3);
}
