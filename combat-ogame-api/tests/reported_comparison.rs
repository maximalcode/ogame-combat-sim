use combat_core::{Combat, ReportedBattle, ReportedSlot, ReportedUnit};
use combat_ogame_api::reports::{ReportedComparisonInput, compare_reported_with};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn direct_stat_comparison_retains_batches_and_labels_its_limits() {
    let input: ReportedComparisonInput = serde_json::from_value(json!({
        "battle": {"attackers":[{"units":{"204":{"count":1,"weapon":99,"shield":100,"hull":400}}}],
                   "defenders":[{"units":{"408":{"count":1,"weapon":1,"shield":10000,"hull":10000}}}],"rapid_fire":false},
        "rapid_fire_basis":"assumed", "source":{"archive_report":null,"battle_timestamp":null},
        "observed":{"winner":"attacker","rounds":1,"attacker_remaining":[{"204":1}],"defender_remaining":[{"408":0}]}
    })).unwrap();
    let mut sizes = Vec::new();
    let result = compare_reported_with(&input, |n| {
        sizes.push(n);
        let actual = ReportedBattle {
            attackers: vec![ReportedSlot {
                units: BTreeMap::from([(
                    204,
                    ReportedUnit {
                        count: 1,
                        weapon: 0,
                        shield: 0.0,
                        hull: 400.0,
                    },
                )]),
            }],
            defenders: vec![ReportedSlot {
                units: BTreeMap::from([(
                    408,
                    ReportedUnit {
                        count: 1,
                        weapon: 0,
                        shield: 0.0,
                        hull: 10000.0,
                    },
                )]),
            }],
            rapid_fire: false,
        };
        (0..n)
            .map(|_| Combat::new().simulate_reported(&actual).unwrap())
            .collect()
    })
    .unwrap();
    assert_eq!(sizes, [50, 150]);
    assert_eq!(result.run_count, 200);
    assert_eq!(result.metrics[0].occurrence_count, Some(0));
    assert!(
        result
            .metrics
            .iter()
            .all(|m| !m.explanation.contains("verified inputs"))
    );
    assert!(result.limitations.contains("modifier provenance"));
    assert_eq!(
        result
            .stages
            .iter()
            .map(|s| s.run_count)
            .collect::<Vec<_>>(),
        [50, 200]
    );
}

#[test]
fn controlled_uncertainty_stops_at_ceiling_and_bad_batches_fail() {
    let input:ReportedComparisonInput=serde_json::from_value(json!({
        "battle":{"attackers":[{"units":{"204":{"count":1,"weapon":0,"shield":0,"hull":1}}}],"defenders":[{"units":{"204":{"count":1,"weapon":0,"shield":0,"hull":1}}}],"rapid_fire":false},
        "rapid_fire_basis":"assumed","source":{"archive_report":null,"battle_timestamp":null},
        "observed":{"winner":"attacker","rounds":null,"attacker_remaining":null,"defender_remaining":null}
    })).unwrap();
    let mut index = 0;
    let mut sizes = Vec::new();
    let result = compare_reported_with(&input, |n| {
        sizes.push(n);
        (0..n)
            .map(|_| {
                let mut sample = Combat::new().simulate_reported(&input.battle).unwrap();
                if index % 20 == 0 {
                    sample.outcome = combat_core::RoundOutcome::AttackersWin;
                }
                index += 1;
                sample
            })
            .collect()
    })
    .unwrap();
    assert_eq!(sizes, [50, 150, 800]);
    assert_eq!(result.run_count, 1000);
    assert_eq!(
        result.metrics[0].status,
        combat_ogame_api::reports::ComparisonStatus::StatisticallyUncertain
    );
    assert!(compare_reported_with(&input, |_| vec![]).is_err());
    let mut invalid = input;
    invalid.observed.attacker_remaining = Some(vec![BTreeMap::from([(204, 2)])]);
    assert!(
        compare_reported_with(&invalid, |_| panic!("invalid evidence must never simulate"))
            .is_err()
    );
}

#[test]
fn missing_simulated_slot_attribution_never_becomes_zero_aggregate_loss() {
    let input:ReportedComparisonInput=serde_json::from_value(json!({
        "battle":{"attackers":[{"units":{"204":{"count":1,"weapon":0,"shield":0,"hull":1}}}],"defenders":[{"units":{"204":{"count":1,"weapon":0,"shield":0,"hull":1}}}],"rapid_fire":false},
        "rapid_fire_basis":"assumed","source":{"archive_report":null,"battle_timestamp":null},
        "observed":{"winner":"draw","rounds":6,"attacker_remaining":[{"204":1}],"defender_remaining":[{"204":1}]}
    })).unwrap();
    let result = compare_reported_with(&input, |n| {
        (0..n)
            .map(|_| {
                let mut sample = Combat::new().simulate_reported(&input.battle).unwrap();
                sample.attacker_slots = None;
                sample
            })
            .collect()
    })
    .unwrap();
    let aggregate = result
        .metrics
        .iter()
        .find(|m| m.name == "attacker.loss_count")
        .unwrap();
    assert_eq!(
        aggregate.status,
        combat_ogame_api::reports::ComparisonStatus::NotAssessable
    );
}
