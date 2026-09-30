use combat_core::{Combat, RoundOutcome, Simulator, economics};
use combat_types::{
    AttackWave, CombatOutcome, DebrisSettings, PartyData, PlanetResources, WaveSequenceRequest,
};
use rand::{SeedableRng, rngs::SmallRng};

fn party(units: &[(u16, u32)]) -> PartyData {
    PartyData {
        entities: units.iter().copied().collect(),
        ..Default::default()
    }
}

fn request() -> WaveSequenceRequest {
    WaveSequenceRequest {
        waves: vec![
            AttackWave {
                attacker: party(&[(206, 12)]),
                ..Default::default()
            },
            AttackWave {
                attacker: party(&[(207, 5)]),
                ..Default::default()
            },
        ],
        defender: party(&[(204, 100), (401, 30)]),
        defender_bonuses: None,
        planet_resources: Some(PlanetResources {
            metal: 100_000,
            crystal: 40_000,
            deuterium: 10_000,
        }),
        debris_settings: DebrisSettings {
            fleet_percentage: 30,
            defence_percentage: 20,
            deuterium: true,
        },
        use_rapid_fire: true,
        collect_compositions: true,
    }
}

#[test]
fn each_trajectory_carries_exact_defender_units_and_fresh_attackers() {
    let mut request = request();
    request.waves[0].attacker = party(&[(214, 1)]);
    request.defender = party(&[(210, 1_000), (408, 1)]);
    request.use_rapid_fire = false;
    let runs = Simulator::new().simulate_wave_sequences(&request, 30);
    assert_eq!(runs.len(), 30);
    assert!(
        runs.iter()
            .any(|run| !run.waves[0].defender_losses.is_empty())
    );
    assert!(
        runs.iter()
            .any(|run| !run.waves[0].defender_remaining.is_empty())
    );
    for run in runs {
        let first = &run.waves[0];
        let second = &run.waves[1];
        assert!(!first.attacker_remaining.is_empty());
        for (&entity, &count) in &first.defender_remaining {
            assert_eq!(
                count,
                second.defender_losses.get(&entity).copied().unwrap_or(0)
                    + second.defender_remaining.get(&entity).copied().unwrap_or(0)
            );
        }
        for entity in second
            .defender_losses
            .keys()
            .chain(second.defender_remaining.keys())
        {
            assert!(first.defender_remaining.contains_key(entity));
        }
        assert!(!second.attacker_remaining.contains_key(&214));
        assert!(!second.attacker_losses.contains_key(&214));
        assert_eq!(
            second.attacker_losses.get(&207).copied().unwrap_or(0)
                + second.attacker_remaining.get(&207).copied().unwrap_or(0),
            5
        );
        assert_eq!(run.final_defender.entities, second.defender_remaining);
        for (&entity, &count) in &request.defender.entities {
            assert_eq!(
                u64::from(count),
                run.totals
                    .defender_losses
                    .get(&entity)
                    .copied()
                    .unwrap_or(0)
                    + u64::from(
                        run.final_defender
                            .entities
                            .get(&entity)
                            .copied()
                            .unwrap_or(0)
                    )
            );
        }
        assert_eq!(
            run.totals.rounds,
            run.waves.iter().map(|w| u64::from(w.rounds)).sum::<u64>()
        );
        assert_eq!(
            run.totals.debris_field.total(),
            run.waves
                .iter()
                .map(|w| w.debris_field.total())
                .sum::<u64>()
        );
        assert_eq!(
            run.totals.loot.total(),
            run.waves.iter().map(|w| w.loot.total()).sum::<u64>()
        );
        assert_eq!(
            run.totals.attacker_profit,
            run.waves.iter().map(|w| w.attacker_profit).sum::<i64>()
        );
        assert_eq!(
            run.totals.defender_profit,
            run.waves.iter().map(|w| w.defender_profit).sum::<i64>()
        );
        assert_eq!(
            request.planet_resources.as_ref().unwrap().total(),
            run.remaining_resources.unwrap().total() + run.totals.loot.total()
        );
    }
}

#[test]
fn one_wave_matches_seeded_single_battle_and_its_economics() {
    let combat = Combat::new();
    let mut request = request();
    request.waves.truncate(1);
    let result = combat.simulate_waves(&request, &mut SmallRng::seed_from_u64(8));
    let single = combat.simulate_single(
        &request.waves[0].attacker,
        &request.defender,
        request.use_rapid_fire,
        true,
        &mut SmallRng::seed_from_u64(8),
    );
    let wave = &result.waves[0];
    assert_eq!(wave.rounds, single.rounds);
    assert_eq!(wave.attacker_remaining, single.attacker_remaining);
    assert_eq!(wave.defender_remaining, single.defender_remaining);
    assert_eq!(wave.attacker_losses, single.attacker_losses);
    assert_eq!(wave.defender_losses, single.defender_losses);
    assert_eq!(
        wave.outcome,
        match single.outcome {
            RoundOutcome::AttackersWin => CombatOutcome::AttackersWin,
            RoundOutcome::DefendersWin => CombatOutcome::DefendersWin,
            RoundOutcome::Draw => CombatOutcome::Draw,
        }
    );
    assert_eq!(
        serde_json::to_value(&wave.round_details).unwrap(),
        serde_json::to_value(&single.round_details).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&wave.round_compositions).unwrap(),
        serde_json::to_value(&single.round_compositions).unwrap()
    );
    let db = combat_types::entities::entity_stats();
    let debris = economics::calculate_debris(
        &single.attacker_losses,
        &single.defender_losses,
        db,
        request.debris_settings,
    );
    let loot = economics::calculate_loot(
        request.planet_resources.as_ref().unwrap(),
        economics::calculate_cargo_capacity(&single.attacker_remaining, db),
    );
    assert_eq!(
        serde_json::to_value(&wave.debris_field).unwrap(),
        serde_json::to_value(&debris).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&wave.loot).unwrap(),
        serde_json::to_value(&loot).unwrap()
    );
    assert_eq!(
        wave.attacker_profit,
        economics::calculate_attacker_profit(&debris, &loot, &single.attacker_losses, db)
    );
    assert_eq!(
        wave.defender_profit,
        economics::calculate_defender_profit(&debris, &single.defender_losses, db)
    );
}

#[test]
fn six_round_draw_does_not_stop_later_missions_or_cap_wave_count() {
    let mut request = request();
    request.defender = party(&[(408, 1)]);
    request.waves = vec![
        AttackWave {
            attacker: party(&[(204, 1)]),
            ..Default::default()
        };
        7
    ];
    request.waves.push(AttackWave {
        attacker: party(&[(214, 1)]),
        ..Default::default()
    });
    let run = Combat::new().simulate_waves(&request, &mut SmallRng::seed_from_u64(8));
    assert_eq!(run.waves.len(), 8);
    for wave in &run.waves[..7] {
        assert_eq!(wave.outcome, CombatOutcome::Draw);
        assert_eq!(wave.rounds, 6);
        assert_eq!(wave.defender_remaining.get(&408), Some(&1));
    }
    assert_eq!(run.waves[7].outcome, CombatOutcome::AttackersWin);
    assert!(run.final_defender.entities.is_empty());
}

#[test]
fn looted_resources_are_removed_before_the_next_mission() {
    let mut request = request();
    request.defender = party(&[]);
    request.waves = vec![
        AttackWave {
            attacker: party(&[(203, 100)]),
            ..Default::default()
        };
        2
    ];
    let run = Combat::new().simulate_waves(&request, &mut SmallRng::seed_from_u64(8));
    assert_eq!(run.waves[0].loot.metal, 50_000);
    assert_eq!(run.waves[1].loot.metal, 25_000);
    assert_eq!(run.totals.loot.metal, 75_000);
    assert_eq!(run.remaining_resources.unwrap().metal, 25_000);
}

#[test]
fn empty_sequences_leave_the_target_unchanged_and_zero_runs_return_none() {
    let mut request = request();
    request.waves.clear();
    let run = Combat::new().simulate_waves(&request, &mut SmallRng::seed_from_u64(8));
    assert!(run.waves.is_empty());
    assert_eq!(run.totals.rounds, 0);
    assert_eq!(run.final_defender.entities, request.defender.entities);
    assert_eq!(
        run.remaining_resources.unwrap().total(),
        request.planet_resources.unwrap().total()
    );
    request.planet_resources = None;
    assert!(
        Simulator::new()
            .simulate_wave_sequences(&request, 0)
            .is_empty()
    );
}

#[test]
fn seeded_sequences_repeat_without_randomizing_carried_target_order() {
    let request = request();
    let combat = Combat::new();
    for seed in 0..20 {
        let first = combat.simulate_waves(&request, &mut SmallRng::seed_from_u64(seed));
        let second = combat.simulate_waves(&request, &mut SmallRng::seed_from_u64(seed));
        assert_eq!(
            serde_json::to_value(first).unwrap(),
            serde_json::to_value(second).unwrap()
        );
    }
}

#[test]
fn defender_class_levels_are_not_compounded_between_missions() {
    let mut request = request();
    request.defender = party(&[(408, 1)]);
    request.defender_bonuses = Some(combat_types::PlayerBonuses {
        player_class: combat_types::PlayerClass::General,
        ..Default::default()
    });
    let mut attacker = party(&[(204, 250)]);
    attacker.technology.weapon = 15;
    request.waves = vec![
        AttackWave {
            attacker,
            ..Default::default()
        };
        3
    ];
    let combat = Combat::new();
    let run = combat.simulate_waves(&request, &mut SmallRng::seed_from_u64(8));
    assert_eq!(run.final_defender.technology, request.defender.technology);
    let mut rng = SmallRng::seed_from_u64(8);
    let mut defender = request.defender.clone();
    for (wave, actual) in request.waves.iter().zip(&run.waves) {
        let expected = combat.simulate_single(
            &wave.attacker,
            &defender.at_effective_levels(request.defender_bonuses.as_ref()),
            request.use_rapid_fire,
            true,
            &mut rng,
        );
        assert_eq!(actual.defender_remaining, expected.defender_remaining);
        assert_eq!(actual.rounds, expected.rounds);
        defender.entities = expected.defender_remaining;
    }
}
