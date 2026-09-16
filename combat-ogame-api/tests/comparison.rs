#[path = "support/comparison_input.rs"]
mod support;
use combat_ogame_api::reports::{
    ComparisonStatus, CompletionInput, CompletionResult, compare_with, complete_candidate,
};
use combat_types::{CombatOutcome, SimulationResult};

fn verified(observed: serde_json::Value) -> combat_ogame_api::reports::VerifiedBattleInput {
    let mut candidate = support::candidate(Some(204), Some(204));
    candidate.observed = Some(observed);
    let CompletionResult::Verified { input } = complete_candidate(&CompletionInput {
        candidate,
        evidence: support::evidence(),
        universe: support::universe(),
    }) else {
        panic!("synthetic input must complete")
    };
    *input
}
fn sample(outcome: CombatOutcome, rounds: u8) -> SimulationResult {
    SimulationResult {
        outcome,
        rounds,
        attacker_losses: std::collections::HashMap::default(),
        defender_losses: std::collections::HashMap::default(),
        attacker_remaining: std::collections::HashMap::default(),
        defender_remaining: std::collections::HashMap::default(),
        debris_field: combat_types::DebrisField::default(),
        loot: combat_types::PlanetResources::default(),
        attacker_profit: 0,
        defender_profit: 0,
        round_details: None,
        round_compositions: None,
        round_compositions_by_slot: None,
        attacker_slots: None,
        defender_slots: None,
    }
}
#[test]
fn central_tied_observation_stops_at_fifty_and_keeps_unknowns_unassessed() {
    let input = verified(serde_json::json!({"winner":"draw", "combat_rounds":6}));
    let result = compare_with(&input, |request| {
        assert_eq!(request.simulations, 50);
        vec![sample(CombatOutcome::Draw, 6); 50]
    })
    .unwrap();
    assert_eq!(result.run_count, 50);
    for name in ["outcome", "rounds"] {
        let metric = result.metrics.iter().find(|m| m.name == name).unwrap();
        assert_eq!(metric.status, ComparisonStatus::Unremarkable);
        assert!((metric.probability.unwrap() - 1.0).abs() < 1e-10);
    }
    assert!(
        result
            .metrics
            .iter()
            .any(|m| m.status == ComparisonStatus::NotAssessable)
    );
}

#[test]
fn absent_outcome_escalates_once_then_stays_suspicious() {
    let input = verified(serde_json::json!({"winner":"attacker"}));
    let mut batches = Vec::new();
    let result = compare_with(&input, |request| {
        batches.push(request.simulations);
        vec![sample(CombatOutcome::Draw, 6); request.simulations as usize]
    })
    .unwrap();
    assert_eq!(batches, [50, 150]);
    let metric = result.metrics.iter().find(|m| m.name == "outcome").unwrap();
    assert_eq!(metric.status, ComparisonStatus::Suspicious);
    assert_eq!(metric.occurrence_count, Some(0));
    assert!(metric.interval.as_ref().unwrap().upper < 0.05);
    assert!((metric.interval.as_ref().unwrap().upper - 0.018_845_326_4).abs() < 1e-8);
    assert!(result.render_text().contains("outcome: suspicious"));
}

#[test]
fn boundary_outcome_retains_every_sample_through_the_ceiling() {
    let input = verified(serde_json::json!({"winner":"attacker"}));
    let mut batches = Vec::new();
    let result = compare_with(&input, |request| {
        batches.push(request.simulations);
        let wins = match request.simulations {
            50 => 2,
            150 => 8,
            800 => 40,
            _ => panic!("unexpected batch"),
        };
        (0..request.simulations)
            .map(|i| {
                sample(
                    if i < wins {
                        CombatOutcome::AttackersWin
                    } else {
                        CombatOutcome::Draw
                    },
                    6,
                )
            })
            .collect()
    })
    .unwrap();
    assert_eq!(batches, [50, 150, 800]);
    assert_eq!(result.run_count, 1000);
    let metric = result.metrics.iter().find(|m| m.name == "outcome").unwrap();
    assert_eq!(metric.occurrence_count, Some(50));
    assert!(
        result
            .render_text()
            .contains("outcome: statistically_uncertain")
    );
    assert_eq!(metric.status, ComparisonStatus::StatisticallyUncertain);
    assert!(metric.interval.as_ref().unwrap().lower < 0.05);
    assert!(metric.interval.as_ref().unwrap().upper > 0.05);
}

#[test]
fn inclusive_numeric_tails_handle_discrete_and_degenerate_distributions() {
    for (observation, expected, runs) in [
        (3, ComparisonStatus::Unremarkable, 50),
        (6, ComparisonStatus::Suspicious, 200),
    ] {
        let input = verified(serde_json::json!({"combat_rounds": observation}));
        let result = compare_with(&input, |r| {
            (0..r.simulations)
                .map(|i| sample(CombatOutcome::Draw, if i % 2 == 0 { 2 } else { 3 }))
                .collect()
        })
        .unwrap();
        let metric = result.metrics.iter().find(|m| m.name == "rounds").unwrap();
        assert_eq!(metric.status, expected);
        assert_eq!(result.run_count, runs);
        let spread = metric.spread.as_ref().unwrap();
        assert!((spread.mean - 2.5).abs() < 1e-10);
        assert!((spread.standard_deviation - 0.5).abs() < 1e-10);
    }
}

#[test]
fn malformed_batches_are_rejected_instead_of_silently_resampled() {
    let input = verified(serde_json::json!({"winner":"attacker"}));
    assert!(compare_with(&input, |_| vec![]).is_err());
}

#[test]
fn losses_keep_ship_counts_separate_from_resource_values_and_remaining_debris() {
    let input = verified(serde_json::json!({
        "combat_rounds":1, "units_lost_attackers":4000,
        "debris_metal_total":900, "debris_metal":12,
        "rounds":[{"round_number":1,"attacker_ship_losses":[{"slot":"A1","ship_type":204,"count":1}],"defender_ship_losses":[]}],
        "repaired_defenses":{"401":1}
    }));
    let result = compare_with(&input, |r| {
        let mut s = sample(CombatOutcome::Draw, 1);
        s.attacker_losses.insert(204, 1);
        s.debris_field.metal = 900;
        vec![s; r.simulations as usize]
    })
    .unwrap();
    for (name, observation) in [
        ("attacker.losses.count", 1),
        ("attacker.losses.204", 1),
        ("attacker.losses.resources", 4000),
        ("generated_debris.metal", 900),
    ] {
        let metric = result.metrics.iter().find(|m| m.name == name).unwrap();
        assert_eq!(metric.observation, serde_json::json!(observation));
        assert_eq!(metric.status, ComparisonStatus::Unremarkable);
    }
    for name in ["remaining_debris.metal", "repaired_defenses"] {
        assert_eq!(
            result
                .metrics
                .iter()
                .find(|m| m.name == name)
                .unwrap()
                .status,
            ComparisonStatus::NotAssessable
        );
    }
}

#[test]
fn incomplete_or_ambiguous_round_evidence_does_not_become_zero_losses() {
    for rounds in [
        serde_json::json!([]),
        serde_json::json!([{"round_number":1,"attacker_ship_losses":null}]),
        serde_json::json!([{"round_number":1,"attacker_ship_losses":[{"slot":null,"ship_type":204,"count":1}]}]),
    ] {
        let input = verified(serde_json::json!({"combat_rounds":1,"rounds":rounds}));
        let result = compare_with(&input, |r| {
            vec![sample(CombatOutcome::Draw, 1); r.simulations as usize]
        })
        .unwrap();
        assert_eq!(result.run_count, 50);
        assert_eq!(
            result
                .metrics
                .iter()
                .find(|m| m.name == "attacker.losses.count")
                .unwrap()
                .status,
            ComparisonStatus::NotAssessable
        );
    }
}

#[test]
fn unknown_historical_debris_settings_prevent_assessment_and_escalation() {
    let mut input = verified(serde_json::json!({"debris_metal_total":900}));
    input
        .assessment_limitations
        .push(combat_ogame_api::reports::AssessmentLimitation {
            metric: "generated_debris".to_owned(),
            location: "universe.settings.debris_fleet".to_owned(),
            explanation: "battle-time setting unknown".to_owned(),
            affects_execution: false,
        });
    let result = compare_with(&input, |r| {
        vec![sample(CombatOutcome::Draw, 6); r.simulations as usize]
    })
    .unwrap();
    assert_eq!(result.run_count, 50);
    let metric = result
        .metrics
        .iter()
        .find(|m| m.name == "generated_debris.metal")
        .unwrap();
    assert_eq!(metric.status, ComparisonStatus::NotAssessable);
    assert_eq!(metric.observation, serde_json::json!(900));
}

#[test]
fn diagnostics_include_starting_stats_and_provenance_but_never_unrecognized_private_text() {
    let mut input = verified(
        serde_json::json!({"winner":"draw","player":"private-secret","repaired_defenses":{"private-secret":42}}),
    );
    input.evidence.fields.insert(
        "private-secret".to_owned(),
        combat_ogame_api::reports::EvidenceRecord {
            source: combat_ogame_api::reports::EvidenceSource::Supplied,
            value: serde_json::json!("private-secret"),
        },
    );
    input
        .evidence
        .fields
        .get_mut("battle.provenance")
        .unwrap()
        .value["game_version"] = serde_json::json!("private-secret");
    let result = compare_with(&input, |r| {
        vec![sample(CombatOutcome::Draw, 6); r.simulations as usize]
    })
    .unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("private-secret"));
    assert!(json.contains("starting_stats"));
    assert!(json.contains("public_metadata"));
    assert!(json.contains("already_effective") || json.contains("researched"));
    let text = result.render_text();
    assert!(text.contains("unremarkable"));
    assert!(text.contains("not_assessable"));
    assert!(text.contains("115")); // LF Weapons 13, applied once
    assert!(!text.contains("private-secret"));
}

#[test]
fn earlier_suspicion_remains_visible_when_another_metric_extends_sampling() {
    let input = verified(serde_json::json!({"winner":"attacker","combat_rounds":1}));
    let result = compare_with(&input, |r| {
        (0..r.simulations)
            .map(|i| {
                let rare = match r.simulations {
                    50 => i == 0,
                    150 => i < 4,
                    _ => i < 20,
                };
                sample(
                    if r.simulations == 800 {
                        CombatOutcome::AttackersWin
                    } else {
                        CombatOutcome::Draw
                    },
                    if rare { 1 } else { 6 },
                )
            })
            .collect()
    })
    .unwrap();
    assert_eq!(result.run_count, 1000);
    assert_eq!(result.stages.len(), 3);
    assert_eq!(
        result.stages[1]
            .metrics
            .iter()
            .find(|m| m.name == "outcome")
            .unwrap()
            .status,
        ComparisonStatus::Suspicious
    );
    assert!(
        result
            .render_text()
            .contains("Earlier suspicious assessment at 200")
    );
}

#[test]
fn zero_rounds_without_loss_evidence_do_not_imply_no_ships_destroyed() {
    let input = verified(serde_json::json!({"combat_rounds":0,"rounds":[]}));
    let result = compare_with(&input, |r| {
        vec![sample(CombatOutcome::Draw, 0); r.simulations as usize]
    })
    .unwrap();
    assert_eq!(
        result
            .metrics
            .iter()
            .find(|m| m.name == "attacker.losses.count")
            .unwrap()
            .status,
        ComparisonStatus::NotAssessable
    );
}
