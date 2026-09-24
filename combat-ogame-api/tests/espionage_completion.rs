use combat_ogame_api::reports::{
    CompletionInput, CompletionResult, EvidenceSource, ReportId, parse_report,
};
use serde_json::json;

#[path = "support/comparison_input.rs"]
#[allow(dead_code)]
mod support;

fn scenario() -> CompletionInput {
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"event_timestamp":1_700_000_000,"failed_ships":false,"failed_defense":false,"failed_research":false},
        "details":{"ships":[{"ship_type":204,"count":12}],"defense":[],
        "research":[{"research_type":109,"level":10},{"research_type":110,"level":10},{"research_type":111,"level":10}]}
    }});
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let mut evidence = support::evidence();
    evidence.participants.get_mut("A1").unwrap().entities = Some([(204, 20)].into());
    evidence.participants.get_mut("D1").unwrap().technology = None;
    CompletionInput {
        candidate: parse_report(&id, &payload.to_string()).unwrap(),
        evidence,
        universe: support::universe(),
    }
}

#[test]
fn revealed_snapshot_and_explicit_attacker_complete_into_a_runnable_scenario() {
    let result = scenario().complete();
    let CompletionResult::Verified { input } = result else {
        panic!("{result:?}")
    };
    assert_eq!(input.request.defender.entities.get(&204), Some(&12));
    assert_eq!(input.request.defender.technology.weapon, 13);
    assert!(input.request.defender_bonuses.is_none());
    assert!(input.observed.is_none());
    assert_eq!(
        input.evidence.fields["D1.ships"].source,
        EvidenceSource::Report
    );
    assert_eq!(
        input.evidence.fields["A1.entities"].source,
        EvidenceSource::Supplied
    );
    assert_eq!(
        input.evidence.fields["snapshot.provenance"].value["event_timestamp"],
        1_700_000_000_u64
    );
    assert_eq!(
        combat_core::Simulator::new()
            .simulate_multiple(&input.request)
            .results
            .len(),
        1
    );
}

#[test]
fn visibility_flags_win_over_candidate_arrays_for_every_combination() {
    for ships in [Some(false), Some(true), None] {
        for defenses in [Some(false), Some(true), None] {
            for research in [Some(false), Some(true), None] {
                let mut value = serde_json::to_value(scenario()).unwrap();
                value["candidate"]["defenders"][0]["espionage_visibility"] = json!({"failed_ships":ships,"failed_defense":defenses,"failed_research":research});
                let input: CompletionInput = serde_json::from_value(value).unwrap();
                let result = input.complete();
                if ships == Some(false) && defenses == Some(false) && research == Some(false) {
                    assert!(matches!(result, CompletionResult::Verified { .. }));
                } else {
                    let CompletionResult::Incomplete { issues } = result else {
                        panic!("visibility must win: {ships:?}/{defenses:?}/{research:?}")
                    };
                    for (flag, field) in [
                        (ships, "ships"),
                        (defenses, "defenses"),
                        (research, "technology.weapon"),
                    ] {
                        if flag != Some(false) {
                            assert!(
                                issues
                                    .iter()
                                    .any(|issue| issue.location == format!("D1.{field}")),
                                "{issues:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn a_snapshot_cannot_be_compared_even_if_an_outcome_is_attached_to_the_candidate() {
    let mut scenario = scenario();
    scenario.candidate.observed = Some(json!({"winner":"attacker"}));
    let CompletionResult::Verified { input } = scenario.complete() else {
        panic!("complete snapshot")
    };
    assert!(input.observed.is_none());
    let result = combat_ogame_api::reports::compare_with(&input, |_| {
        panic!("no simulation without an observed battle")
    });
    assert!(result.unwrap_err().contains("observed combat report"));
}

#[test]
fn undocumented_espionage_values_are_sanitized_evidence_and_never_stat_checks() {
    let id = ReportId::parse("sr-en-1-0000000000000000000000000000000000000000").unwrap();
    let payload = json!({"RESULT_CODE":1000,"RESULT_DATA":{
        "generic":{"failed_ships":false,"failed_defense":false,"failed_research":false},
        "details":{"ships":[{"ship_type":204,"count":12}],"defense":[],
          "research":[{"research_type":109,"level":10},{"research_type":110,"level":10},{"research_type":111,"level":10}],
          "combatInformation":{"weapon":999.0,"shield":456.0,"armor":123.0,"player_name":"secret"},
          "lifeformBonuses":{"BaseStatsBooster":{"204":{"weapon":321.0,"secret":"private"}}}}
    }});
    let mut input = scenario();
    input.candidate = parse_report(&id, &payload.to_string()).unwrap();
    let sanitized = serde_json::to_value(&input.candidate).unwrap();
    assert_eq!(
        sanitized["defenders"][0]["reported_combat_information"]["weapon"],
        999.0
    );
    assert!(!sanitized.to_string().contains("secret"));
    let CompletionResult::Verified { input } = input.complete() else {
        panic!("undocumented numbers must not become combat-report stats")
    };
    assert_eq!(input.request.defender.technology.weapon, 13);
    assert_eq!(
        input.evidence.fields["D1.reported_combat_information"].value["weapon"],
        999.0
    );
    assert_eq!(
        input.evidence.fields["D1.reported_base_stats_booster"].value["204"]["weapon"],
        321.0
    );
}

#[test]
fn missing_attacker_and_modifier_evidence_is_never_defaulted() {
    for field in ["entities", "technology", "lifeform"] {
        let mut value = serde_json::to_value(scenario()).unwrap();
        value["evidence"]["participants"]["A1"][field] = serde_json::Value::Null;
        let input: CompletionInput = serde_json::from_value(value).unwrap();
        let CompletionResult::Incomplete { issues } = input.complete() else {
            panic!("missing attacker {field}")
        };
        assert!(
            issues.iter().any(|i| i.location == format!("A1.{field}")),
            "{issues:?}"
        );
    }
    for field in ["player_class", "alliance_class", "lifeform"] {
        let mut value = serde_json::to_value(scenario()).unwrap();
        value["evidence"]["participants"]["D1"][field] = serde_json::Value::Null;
        let input: CompletionInput = serde_json::from_value(value).unwrap();
        let CompletionResult::Incomplete { issues } = input.complete() else {
            panic!("missing defender {field}")
        };
        assert!(
            issues.iter().any(|i| i.location == format!("D1.{field}")),
            "{issues:?}"
        );
    }
}

#[test]
fn supplied_hidden_defenses_cannot_overwrite_revealed_ships() {
    let mut input = scenario();
    input.candidate.defenders[0]
        .espionage_visibility
        .as_mut()
        .unwrap()
        .failed_defense = Some(true);
    input.evidence.participants.get_mut("D1").unwrap().entities =
        Some([(204, 12), (401, 2)].into());
    let CompletionResult::Verified { input: verified } = input.complete() else {
        panic!("hidden defenses can be supplied")
    };
    assert_eq!(verified.request.defender.entities.get(&401), Some(&2));
    assert_eq!(
        verified.evidence.fields["D1.defenses"].source,
        EvidenceSource::Supplied
    );
    input.evidence.participants.get_mut("D1").unwrap().entities =
        Some([(204, 13), (401, 2)].into());
    let CompletionResult::Incomplete { issues } = input.complete() else {
        panic!("conflicting revealed fleet")
    };
    assert!(
        issues
            .iter()
            .any(|i| i.kind == combat_ogame_api::reports::FieldIssueKind::Contradictory)
    );
}

#[test]
fn supplied_effective_technology_is_checked_against_revealed_research_once() {
    let mut input = scenario();
    input
        .evidence
        .participants
        .get_mut("D1")
        .unwrap()
        .technology = Some(combat_ogame_api::reports::TechnologyEvidence {
        basis: combat_ogame_api::reports::TechnologyBasis::AlreadyEffective,
        weapon: 13,
        shield: 13,
        armour: 13,
    });
    let CompletionResult::Verified { input: verified } = input.complete() else {
        panic!("10 research plus 3 class levels")
    };
    assert_eq!(verified.request.defender.technology.weapon, 13);
    input
        .evidence
        .participants
        .get_mut("D1")
        .unwrap()
        .technology
        .as_mut()
        .unwrap()
        .weapon = 16;
    assert!(matches!(
        input.complete(),
        CompletionResult::Incomplete { .. }
    ));
}

#[test]
fn empty_revealed_defender_is_known_and_can_run() {
    let mut input = scenario();
    input.candidate.defenders[0].ships = Some(std::collections::BTreeMap::new());
    input.candidate.defenders[0].entities = Some(std::collections::BTreeMap::new());
    let result = input.complete();
    let CompletionResult::Verified { input } = result else {
        panic!("{result:?}")
    };
    assert!(input.request.defender.entities.is_empty());
}

#[test]
fn missing_research_fields_and_unsupported_basis_remain_targeted_issues() {
    let mut input = scenario();
    input.candidate.defenders[0].technology.shield = None;
    let CompletionResult::Incomplete { issues } = input.complete() else {
        panic!("missing shielding research")
    };
    assert!(issues.iter().any(|i| i.location == "D1.technology.shield"));
    input.candidate.defenders[0].technology.shield = Some(10);
    input.candidate.defenders[0].technology.basis = "undocumented".into();
    assert!(matches!(
        input.complete(),
        CompletionResult::Incomplete { .. }
    ));
}

#[test]
fn snapshot_and_current_universe_times_remain_separate() {
    let mut input = scenario();
    input.universe.current = Some(true);
    input.universe.acknowledged_current = Some(true);
    let CompletionResult::Verified { input } = input.complete() else {
        panic!("acknowledged current universe")
    };
    assert_eq!(
        input.evidence.fields["snapshot.provenance"].value["event_timestamp"],
        1_700_000_000_u64
    );
    assert_eq!(
        input.evidence.fields["universe"].value["source_timestamp"],
        1_700_000_100_u64
    );
    assert!(
        input
            .assessment_limitations
            .iter()
            .any(|l| l.metric == "generated_debris")
    );
}
