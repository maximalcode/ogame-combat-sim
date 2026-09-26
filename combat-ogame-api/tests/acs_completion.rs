#[path = "support/comparison_input.rs"]
mod support;
use combat_ogame_api::reports::{CompletionInput, CompletionResult};

#[test]
fn every_participant_retains_its_own_effective_modifiers_and_ledger() {
    let mut candidate = support::candidate(Some(204), Some(204));
    let mut evidence = support::evidence();
    for (slot, attackers) in [("A2", true), ("D2", false)] {
        let mut participant = support::participant(slot, Some(204));
        participant.character_class_id = Some(0);
        participant.alliance_class_id = Some(0);
        participant.technology.weapon = Some(10);
        participant.technology.shield = Some(10);
        participant.technology.armour = Some(10);
        let mut supplied = evidence.participants["A1"].clone();
        supplied.player_class = Some(combat_types::PlayerClass::None);
        supplied.alliance_class = Some(combat_types::AllianceClass::None);
        evidence.participants.insert(slot.into(), supplied);
        if attackers {
            candidate.attackers.push(participant);
        } else {
            candidate.defenders.push(participant);
        }
    }
    let result = CompletionInput {
        candidate,
        evidence,
        universe: support::universe(),
    }
    .complete();
    let CompletionResult::Verified { input } = result else {
        panic!("{result:?}")
    };
    for (slots, prefix) in [
        (input.request.attacker_slots.as_ref().unwrap(), "A"),
        (input.request.defender_slots.as_ref().unwrap(), "D"),
    ] {
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].id, format!("{prefix}1"));
        assert_eq!(slots[1].id, format!("{prefix}2"));
        assert_eq!(slots[0].data.technology.weapon, 13);
        assert_eq!(slots[1].data.technology.weapon, 10);
        assert!(
            input
                .evidence
                .fields
                .contains_key(&format!("{prefix}2.entities"))
        );
    }
    assert!(input.request.attacker_bonuses.is_none());
    assert!(input.request.defender_bonuses.is_none());
}

#[test]
fn ambiguous_losses_keep_side_totals_without_fabricating_participant_losses() {
    let mut candidate = support::candidate(Some(204), Some(204));
    candidate
        .attackers
        .push(support::participant("A2", Some(204)));
    candidate.observed = Some(serde_json::json!({
        "winner":"draw", "combat_rounds":1,
        "rounds":[{"round_number":1,"attacker_ship_losses":[
            {"slot":null,"ship_type":204,"count":2},
            {"slot":null,"ship_type":204,"count":3}
        ],"defender_ship_losses":[]}]
    }));
    let mut evidence = support::evidence();
    evidence
        .participants
        .insert("A2".into(), evidence.participants["A1"].clone());
    let CompletionResult::Verified { input } = (CompletionInput {
        candidate,
        evidence,
        universe: support::universe(),
    })
    .complete() else {
        panic!("completion failed")
    };
    let result = combat_ogame_api::reports::compare_with(&input, |request| {
        vec![sample(); request.simulations as usize]
    })
    .unwrap();
    let aggregate = result
        .metrics
        .iter()
        .find(|m| m.name == "attacker.losses.count")
        .unwrap();
    assert_eq!(aggregate.observation, 5);
    assert_eq!(
        aggregate.status,
        combat_ogame_api::reports::ComparisonStatus::Unremarkable
    );
    let participant = result
        .metrics
        .iter()
        .find(|m| m.name == "A1.losses.count")
        .unwrap();
    assert_eq!(
        participant.status,
        combat_ogame_api::reports::ComparisonStatus::NotAssessable
    );
    assert!(participant.explanation.contains("attribution"));
}

fn sample() -> combat_types::SimulationResult {
    combat_types::SimulationResult {
        outcome: combat_types::CombatOutcome::Draw,
        rounds: 1,
        attacker_losses: [(204, 5)].into(),
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
fn diagnostics_show_each_participants_verified_stats_and_redact_private_strings() {
    let mut candidate = support::candidate(Some(204), Some(204));
    let mut second = support::participant("A2", Some(204));
    second.reported_unit_stats =
        Some(serde_json::json!({"204":{"weapon":140,"shield":28,"armor":1120}}));
    candidate.attackers.push(second);
    let mut evidence = support::evidence();
    let mut supplied = evidence.participants["A1"].clone();
    supplied.lifeform = Some(
        [(
            204,
            combat_ogame_api::reports::PartialLifeformBonus {
                weapon: Some(50.0),
                shield: Some(50.0),
                armour: Some(50.0),
                ..Default::default()
            },
        )]
        .into(),
    );
    evidence.participants.insert("A2".into(), supplied);
    let CompletionResult::Verified { mut input } = (CompletionInput {
        candidate,
        evidence,
        universe: support::universe(),
    })
    .complete() else {
        panic!("completion failed")
    };
    input.evidence.fields.insert(
        "A2.private-name".into(),
        combat_ogame_api::reports::EvidenceRecord {
            source: combat_ogame_api::reports::EvidenceSource::Report,
            value: serde_json::json!("private-secret"),
        },
    );
    let result =
        combat_ogame_api::reports::compare_with(&input, |r| vec![sample(); r.simulations as usize])
            .unwrap();
    let json = serde_json::to_value(&result).unwrap();
    let stats = json["diagnostics"]["starting_stats"].as_array().unwrap();
    assert_eq!(stats.len(), 3);
    assert_eq!(stats[0]["slot"], "A1");
    assert_eq!(stats[0]["weapon"], 115);
    assert_eq!(stats[1]["slot"], "A2");
    assert_eq!(stats[1]["weapon"], 140);
    assert_eq!(stats[1]["hull"], 1120.0);
    assert!(!json["diagnostics"]["evidence"]["fields"]["A2.lifeform"].is_null());
    assert!(!result.render_text().contains("private-secret"));
}

#[path = "support/acs_input.rs"]
mod imported;

#[test]
fn parsed_acs_owner_mapping_controls_only_participant_assessment() {
    for owners in [
        [Some(11), Some(12), Some(11), Some(12)],
        [Some(11), Some(11), Some(12), Some(12)],
        [None, None, None, None],
    ] {
        let artifact = imported::artifact(owners);
        let CompletionResult::Verified { input } = artifact.complete() else {
            panic!("completion failed")
        };
        let result = combat_ogame_api::reports::compare_with(&input, |r| {
            let mut sample = sample();
            sample.defender_losses = [(204, 5)].into();
            sample.debris_field.metal = 9000;
            sample.debris_field.crystal = 3000;
            // Deliberately reverse result order: identity, not array position,
            // selects the participant's simulation evidence.
            let slots = |prefix: &str| {
                Some(vec![
                    combat_types::SlotResult {
                        slot_id: format!("{prefix}2"),
                        initial: [(204, 20)].into(),
                        losses: [(204, 3)].into(),
                        remaining: [(204, 17)].into(),
                    },
                    combat_types::SlotResult {
                        slot_id: format!("{prefix}1"),
                        initial: [(204, 20)].into(),
                        losses: [(204, 2)].into(),
                        remaining: [(204, 18)].into(),
                    },
                ])
            };
            sample.attacker_slots = slots("A");
            sample.defender_slots = slots("D");
            vec![sample; r.simulations as usize]
        })
        .unwrap();
        assert_eq!(result.run_count, 50);
        for name in [
            "outcome",
            "rounds",
            "attacker.losses.count",
            "defender.losses.count",
            "attacker.losses.resources",
            "generated_debris.metal",
        ] {
            assert_eq!(
                result
                    .metrics
                    .iter()
                    .find(|m| m.name == name)
                    .unwrap()
                    .status,
                combat_ogame_api::reports::ComparisonStatus::Unremarkable,
                "{name}"
            );
        }
        for slot in ["A1", "A2", "D1", "D2"] {
            let metric = result
                .metrics
                .iter()
                .find(|m| m.name == format!("{slot}.losses.count"))
                .unwrap();
            if owners[0].is_some() && owners[0] != owners[1] {
                assert_eq!(
                    metric.status,
                    combat_ogame_api::reports::ComparisonStatus::Unremarkable
                );
                assert_eq!(metric.observation, if slot.ends_with('1') { 2 } else { 3 });
            } else {
                assert_eq!(
                    metric.status,
                    combat_ogame_api::reports::ComparisonStatus::NotAssessable
                );
            }
        }
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("private-name"));
        assert!(!json.contains("fleet_owner"));
    }
}

#[test]
fn attributed_losses_cannot_exceed_the_owning_participants_fleet() {
    let mut artifact = imported::artifact([Some(11), Some(12), Some(13), Some(14)]);
    artifact.candidate.observed.as_mut().unwrap()["rounds"][0]["attacker_ship_losses"][0]["count"] =
        21.into();
    let CompletionResult::Verified { input } = artifact.complete() else {
        panic!("completion failed")
    };
    let result =
        combat_ogame_api::reports::compare_with(&input, |r| vec![sample(); r.simulations as usize])
            .unwrap();
    assert_eq!(
        result
            .metrics
            .iter()
            .find(|m| m.name == "attacker.losses.count")
            .unwrap()
            .status,
        combat_ogame_api::reports::ComparisonStatus::NotAssessable
    );
}

#[test]
fn conflicting_local_identities_are_rejected_without_echoing_private_names() {
    let mut artifact = imported::artifact([Some(11), Some(12), Some(13), Some(14)]);
    artifact.candidate.attackers[1].slot = "private-name".into();
    let result = artifact.complete();
    let CompletionResult::Incomplete { issues } = &result else {
        panic!("identity conflict accepted")
    };
    assert!(issues.iter().any(|issue| issue.location == "A2.slot"));
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("private-name")
    );
}

#[test]
fn participants_beyond_engine_slot_capacity_are_rejected_before_identities_wrap() {
    let mut artifact = imported::artifact([Some(11), Some(12), Some(13), Some(14)]);
    let participant = artifact.candidate.attackers[0].clone();
    let evidence = artifact.evidence.participants["A1"].clone();
    artifact.candidate.attackers.clear();
    for number in 1..=256 {
        let mut participant = participant.clone();
        participant.slot = format!("A{number}");
        artifact
            .evidence
            .participants
            .insert(participant.slot.clone(), evidence.clone());
        artifact.candidate.attackers.push(participant);
    }
    let CompletionResult::Incomplete { issues } = artifact.complete() else {
        panic!("slot identities would wrap")
    };
    assert!(issues.iter().any(|issue| issue.location == "attackers"
        && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unsupported));
}
