use combat_ogame_api::reports::{
    Candidate, CompletionEvidence, CompletionInput, CompletionResult, EvidenceSource, Participant,
    ParticipantEvidence, PinnedUniverse, PinnedUniverseSettings, Provenance, TechnologyBasis,
    TechnologyCandidate, TechnologyEvidence, complete_candidate,
};
use combat_ogame_api::{
    OGameClient, Universe, parse_server_data,
    reports::{pinned_universe_from_server_data, resolve_current_universe},
};
use combat_types::{AllianceClass, PlayerClass};
use std::collections::BTreeMap;

fn universe() -> PinnedUniverse {
    PinnedUniverse {
        community: "en".to_owned(),
        universe: 1,
        settings: PinnedUniverseSettings {
            galaxies: Some(9),
            systems: Some(499),
            donut_galaxy: Some(true),
            donut_systems: Some(true),
            fleet_speed: Some(1),
            rapid_fire: Some(true),
            debris_fleet: Some(30),
            debris_defence: Some(0),
            debris_deuterium: Some(false),
            deuterium_save_factor: Some(0),
        },
        source: EvidenceSource::PublicMetadata,
        source_timestamp: Some(1_700_000_100),
        source_version: Some("13.0.1".to_owned()),
        current: Some(false),
        acknowledged_current: Some(false),
    }
}

fn participant(slot: &str, entity: Option<u16>) -> Participant {
    Participant {
        slot: slot.to_owned(),
        entities: entity.map(|entity| BTreeMap::from([(entity, 20)])),
        ships: None,
        defenses: None,
        technology: TechnologyCandidate {
            basis: "reported_combat_bonus_divided_by_ten".to_owned(),
            weapon: Some(13),
            shield: Some(13),
            armour: Some(13),
        },
        character_class_id: Some(2),
        alliance_class_id: Some(2),
        reported_base_stats_booster: Some(serde_json::json!({"204":{"weapon":1.2}})),
        reported_unit_stats: None,
    }
}

fn candidate(attacker: Option<u16>, defender: Option<u16>) -> Candidate {
    Candidate {
        schema_version: 1,
        report_kind: combat_ogame_api::reports::ReportKind::Combat,
        provenance: Provenance {
            source: "community_api_proxy".to_owned(),
            community: "en".to_owned(),
            universe: 1,
            event_timestamp: Some(1_700_000_000),
            game_version: Some("13.0.1".to_owned()),
        },
        attackers: vec![participant("A1", attacker)],
        defenders: vec![participant("D1", defender)],
        observed: Some(serde_json::json!({"winner":"attacker"})),
        planet_resources: None,
        loot_percentage: Some(50),
        review_required: Vec::new(),
    }
}

fn evidence() -> CompletionEvidence {
    let technology = TechnologyEvidence {
        basis: TechnologyBasis::Researched,
        weapon: 10,
        shield: 10,
        armour: 10,
    };
    CompletionEvidence {
        participants: BTreeMap::from([
            (
                "A1".to_owned(),
                ParticipantEvidence {
                    technology: Some(technology.clone()),
                    player_class: Some(PlayerClass::General),
                    alliance_class: Some(AllianceClass::Warrior),
                    lifeform: Some(BTreeMap::new()),
                    ..Default::default()
                },
            ),
            (
                "D1".to_owned(),
                ParticipantEvidence {
                    technology: Some(technology),
                    player_class: Some(PlayerClass::General),
                    alliance_class: Some(AllianceClass::Warrior),
                    lifeform: Some(BTreeMap::new()),
                    ..Default::default()
                },
            ),
        ]),
    }
}

#[test]
fn public_server_metadata_resolves_to_a_current_pinned_universe() {
    let server = parse_server_data(include_str!("fixtures/serverData.xml")).unwrap();
    let resolved = pinned_universe_from_server_data("en", 123, &server).unwrap();

    assert_eq!(resolved.community, "en");
    assert_eq!(resolved.universe, 123);
    assert_eq!(resolved.settings.galaxies, Some(9));
    assert_eq!(resolved.settings.systems, Some(499));
    assert_eq!(resolved.settings.fleet_speed, Some(2));
    assert_eq!(resolved.settings.rapid_fire, Some(true));
    assert_eq!(resolved.settings.debris_fleet, Some(70));
    assert_eq!(resolved.settings.debris_defence, Some(30));
    assert_eq!(resolved.settings.debris_deuterium, Some(true));
    assert_eq!(resolved.settings.deuterium_save_factor, Some(80));
    assert_eq!(resolved.source, EvidenceSource::PublicMetadata);
    assert_eq!(resolved.source_timestamp, Some(1_700_000_000));
    assert_eq!(resolved.source_version.as_deref(), Some("13.0.0"));
    assert_eq!(resolved.current, Some(true));
    assert_eq!(resolved.acknowledged_current, Some(false));
}

#[test]
fn public_server_metadata_rejects_a_mismatched_identity() {
    let server = parse_server_data(include_str!("fixtures/serverData.xml")).unwrap();
    let error = pinned_universe_from_server_data("de", 123, &server).unwrap_err();

    assert!(error.to_string().contains("identity"));
}

#[test]
fn public_server_metadata_rejects_non_integral_or_out_of_range_units() {
    let mut server = parse_server_data(include_str!("fixtures/serverData.xml")).unwrap();
    server.debris_factor = 0.705;
    let error = pinned_universe_from_server_data("en", 123, &server).unwrap_err();

    assert!(error.to_string().contains("debris_factor"));
}

#[test]
fn public_server_metadata_preserves_missing_output_only_setting_as_unknown() {
    let xml = include_str!("fixtures/serverData.xml")
        .replace("  <deuteriumInDebris>1</deuteriumInDebris>\n", "");
    let server = parse_server_data(&xml).unwrap();
    let pinned = pinned_universe_from_server_data("en", 123, &server).unwrap();

    assert_eq!(pinned.settings.debris_deuterium, None);
}

#[test]
fn public_rapid_fire_setting_reaches_the_completed_request() {
    let mut server = parse_server_data(include_str!("fixtures/serverData.xml")).unwrap();
    server.number = 1;
    server.rapid_fire = false;
    let pinned = pinned_universe_from_server_data("en", 1, &server).unwrap();
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: PinnedUniverse {
            acknowledged_current: Some(true),
            ..pinned
        },
    });
    let CompletionResult::Verified { input } = result else {
        panic!("expected verified input");
    };
    assert!(!input.request.use_rapid_fire);
}

#[test]
fn missing_debris_settings_keep_execution_verified_with_metric_limitations() {
    let xml = include_str!("fixtures/serverData.xml")
        .replace("  <deuteriumInDebris>1</deuteriumInDebris>\n", "");
    let server = parse_server_data(&xml).unwrap();
    let pinned = pinned_universe_from_server_data("en", 123, &server)
        .expect("missing output-only setting should remain resolvable");
    let pinned = PinnedUniverse {
        settings: PinnedUniverseSettings {
            debris_fleet: None,
            debris_defence: None,
            ..pinned.settings
        },
        ..pinned
    };
    let mut candidate = candidate(Some(204), Some(401));
    candidate.provenance.universe = 123;
    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence: evidence(),
        universe: PinnedUniverse {
            current: Some(false),
            acknowledged_current: Some(false),
            ..pinned
        },
    });
    let CompletionResult::Verified { input } = result else {
        panic!("missing debris settings must not block combat execution");
    };
    assert!(input.assessment_limitations.iter().any(|limitation| {
        limitation.metric == "generated_debris"
            && limitation.location == "universe.settings.debris_deuterium"
            && !limitation.affects_execution
    }));
    for field in ["debris_fleet", "debris_defence"] {
        assert!(input.assessment_limitations.iter().any(|limitation| {
            limitation.metric == "generated_debris"
                && limitation.location == format!("universe.settings.{field}")
                && !limitation.affects_execution
        }));
    }
    let universe_evidence = &input.evidence.fields["universe"].value["settings"];
    assert!(universe_evidence["debris_fleet"].is_null());
    assert!(universe_evidence["debris_defence"].is_null());
    assert!(universe_evidence["debris_deuterium"].is_null());
}

#[tokio::test]
async fn current_resolver_uses_the_public_clients_existing_cache_path() {
    let cache = std::env::temp_dir().join(format!(
        "combat-ogame-api-resolution-{}",
        std::process::id()
    ));
    let directory = cache.join("s123-en");
    std::fs::create_dir_all(&directory).unwrap();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let xml = include_str!("fixtures/serverData.xml").replace(
        "timestamp=\"1700000000\"",
        &format!("timestamp=\"{timestamp}\""),
    );
    std::fs::write(directory.join("serverData.xml"), xml).unwrap();

    let mut report = candidate(Some(204), Some(401));
    report.provenance.universe = 123;
    let client = OGameClient::new(Universe::new("s123-en").unwrap(), &cache).unwrap();
    let resolved = resolve_current_universe(&report, &client).await.unwrap();

    assert_eq!(resolved.universe, 123);
    assert_eq!(resolved.source, EvidenceSource::PublicMetadata);
    let _ = std::fs::remove_dir_all(cache);
}

#[test]
fn acknowledged_current_settings_keep_execution_verified_but_limit_historical_debris() {
    let mut pinned = universe();
    pinned.current = Some(true);
    pinned.acknowledged_current = Some(true);
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: pinned,
    });
    let CompletionResult::Verified { input } = result else {
        panic!("acknowledged current settings should execute");
    };

    assert_eq!(input.assessment_limitations.len(), 5);
    for metric in [
        "generated_debris",
        "moon_chance",
        "recyclers_needed",
        "attacker_profit",
        "defender_profit",
    ] {
        let limitation = input
            .assessment_limitations
            .iter()
            .find(|limitation| limitation.metric == metric)
            .unwrap_or_else(|| panic!("missing limitation for {metric}"));
        assert_eq!(limitation.location, "universe.settings.debris_fleet");
        assert!(!limitation.affects_execution);
    }
}

#[test]
fn a_complete_single_report_becomes_an_effective_request_with_a_separate_ledger() {
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: universe(),
    });
    let CompletionResult::Verified { input } = result else {
        panic!("expected verified input");
    };
    assert_eq!(input.request.attacker.entities[&204], 20);
    assert_eq!(input.request.attacker.technology.weapon, 13);
    assert_eq!(input.request.defender.technology.weapon, 13);
    assert!(input.request.attacker_bonuses.is_none());
    assert!(input.request.defender_bonuses.is_none());
    assert_eq!(
        input.observed,
        Some(serde_json::json!({"winner":"attacker"}))
    );
    assert_eq!(
        input.evidence.fields["A1.entities"].source,
        EvidenceSource::Report
    );
    assert_eq!(
        input.evidence.fields["A1.technology"].source,
        EvidenceSource::Supplied
    );
    assert_eq!(
        input.evidence.fields["universe"].source,
        EvidenceSource::PublicMetadata
    );
    assert!(
        !serde_json::to_value(&input.request)
            .unwrap()
            .to_string()
            .contains("winner")
    );
    let simulated = combat_core::Simulator::new().simulate_multiple(&input.request);
    assert_eq!(simulated.simulations, 1);
    assert_eq!(simulated.results.len(), 1);
}

#[test]
fn completion_returns_all_actionable_issues_without_a_request() {
    let mut pinned = universe();
    pinned.current = Some(true);
    pinned.acknowledged_current = Some(false);
    pinned.settings.debris_fleet = None;
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(None, None),
        evidence: CompletionEvidence::default(),
        universe: pinned,
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("expected incomplete result");
    };
    assert!(issues.iter().any(|issue| issue.location == "A1.entities"));
    assert!(issues.iter().any(|issue| issue.location == "D1.entities"));
    assert!(issues.iter().any(|issue| issue.location == "A1.technology"));
    assert!(
        issues
            .iter()
            .any(|issue| issue.location == "universe.acknowledged_current")
    );
}

#[test]
fn observed_and_supplied_values_cannot_disagree_and_reported_stats_are_checked() {
    let mut report = participant("A1", Some(204));
    report.reported_unit_stats = Some(serde_json::json!({"204":{"weapon":999}}));
    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers = vec![report];
    let mut evidence = evidence();
    evidence.participants.get_mut("A1").unwrap().entities = Some(BTreeMap::from([(204, 21)]));
    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("expected incomplete result");
    };
    assert!(
        issues
            .iter()
            .any(|issue| issue.kind == combat_ogame_api::reports::FieldIssueKind::Contradictory)
    );
    assert!(
        issues.iter().any(
            |issue| issue.kind == combat_ogame_api::reports::FieldIssueKind::ReportStatMismatch
        )
    );
}

#[test]
fn already_effective_technology_is_used_directly_and_lifeform_units_are_explicit() {
    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers[0].technology = TechnologyCandidate {
        basis: "already_effective".to_owned(),
        weapon: Some(13),
        shield: Some(13),
        armour: Some(13),
    };
    let mut evidence = evidence();
    evidence.participants.get_mut("A1").unwrap().technology = Some(TechnologyEvidence {
        basis: TechnologyBasis::AlreadyEffective,
        weapon: 13,
        shield: 13,
        armour: 13,
    });
    evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .lifeform
        .as_mut()
        .unwrap()
        .insert(
            204,
            combat_ogame_api::reports::PartialLifeformBonus {
                weapon: Some(50.0),
                shield: Some(50.0),
                armour: Some(50.0),
                ..Default::default()
            },
        );
    candidate.attackers[0].reported_unit_stats = Some(serde_json::json!({
        // Light Fighter base stats are weapon 50, shield 10, armour 4000.
        // Effective technology 13 and +50% lifeform research therefore yield
        // 140 weapon, 28 shield and 1120 combat hull.
        "204": {"weapon": 140, "shield": 28, "armor": 1120}
    }));
    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence,
        universe: universe(),
    });
    let CompletionResult::Verified { input } = result else {
        panic!("expected verified input");
    };
    assert_eq!(input.request.attacker.technology.weapon, 13);
    assert!((input.request.attacker.lifeform.get(204).weapon - 50.0).abs() < f32::EPSILON);
    assert!(input.request.attacker_bonuses.is_none());
    // The provider booster remains report evidence; it was never interpreted
    // as the independently supplied simulator percentage.
    assert_eq!(
        input.evidence.fields["A1.reported_base_stats_booster"].source,
        EvidenceSource::Report
    );
}

#[test]
fn partial_lifeform_entries_report_each_missing_combat_percentage() {
    let mut evidence = evidence();
    let a1 = evidence.participants.get_mut("A1").unwrap();
    let partial: serde_json::Value = serde_json::json!({
        "204": { "weapon": 50.0 }
    });
    a1.lifeform = Some(serde_json::from_value(partial).unwrap());
    a1.lifeform.as_mut().unwrap().insert(
        206,
        combat_ogame_api::reports::PartialLifeformBonus {
            weapon: Some(0.0),
            shield: Some(0.0),
            armour: Some(0.0),
            ..Default::default()
        },
    );

    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers[0].entities = Some(BTreeMap::from([(204, 20), (206, 1)]));
    candidate.attackers[0].reported_unit_stats = Some(serde_json::json!({
        "204": {"weapon": 140},
        "206": {"weapon": 999}
    }));
    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("partial lifeform combat evidence must not produce a request");
    };

    for stat in ["shield", "armour"] {
        assert!(issues.iter().any(|issue| {
            issue.location == format!("A1.lifeform.204.{stat}")
                && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
        }));
    }
    assert!(!issues.iter().any(|issue| {
        issue.location == "A1.lifeform.204.weapon"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
    }));
    assert!(!issues.iter().any(|issue| {
        issue.location == "A1.reported_unit_stats.204.weapon"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::ReportStatMismatch
    }));
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.reported_unit_stats.206.weapon"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::ReportStatMismatch
    }));
    assert!(issues.iter().all(|issue| issue.location != "D1.lifeform"));
}

#[test]
fn known_stat_conflict_is_reported_when_another_stat_needs_lifeform_evidence() {
    let mut evidence = evidence();
    evidence.participants.get_mut("A1").unwrap().lifeform = Some(BTreeMap::from([(
        204,
        combat_ogame_api::reports::PartialLifeformBonus {
            weapon: Some(50.0),
            shield: None,
            armour: None,
            ..Default::default()
        },
    )]));

    let mut report = participant("A1", Some(204));
    report.reported_unit_stats = Some(serde_json::json!({
        // Effective Weapons 13 plus the known +50% weapon bonus gives 140.
        "204": {"weapon": 999, "shield": 28, "armor": 1120}
    }));
    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers = vec![report];

    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("incomplete lifeform evidence must not produce a request");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.lifeform.204.shield"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
    }));
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.lifeform.204.armour"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
    }));
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.reported_unit_stats.204.weapon"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::ReportStatMismatch
    }));
}

#[test]
fn omitted_lifeform_map_does_not_create_stat_mismatches_from_zero_bonus_guess() {
    let mut report = participant("A1", Some(204));
    report.reported_unit_stats = Some(serde_json::json!({
        // Without a confirmed lifeform map, 140 cannot be compared with a
        // guessed zero-bonus reconstruction of 115.
        "204": {"weapon": 140}
    }));
    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers = vec![report];
    let mut completion_evidence = evidence();
    completion_evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .lifeform = None;

    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence: completion_evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("unknown lifeform evidence must not produce a request");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.lifeform"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unknown
    }));
    assert!(!issues.iter().any(|issue| {
        issue.location == "A1.reported_unit_stats.204.weapon"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::ReportStatMismatch
    }));
}

#[test]
fn null_lifeform_combat_percentages_are_missing_and_all_participant_issues_are_returned() {
    let mut evidence = evidence();
    evidence.participants.get_mut("A1").unwrap().lifeform = Some(BTreeMap::from([(
        204,
        serde_json::from_value(serde_json::json!({
            "weapon": 0.0,
            "shield": null,
            "armour": null
        }))
        .unwrap(),
    )]));
    evidence.participants.get_mut("D1").unwrap().lifeform = Some(BTreeMap::from([(
        401,
        serde_json::from_value(serde_json::json!({})).unwrap(),
    )]));

    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("incomplete lifeform entries must not produce a request");
    };

    for (slot, entity, stats) in [
        ("A1", 204, ["shield", "armour"].as_slice()),
        ("D1", 401, ["weapon", "shield", "armour"].as_slice()),
    ] {
        for stat in stats {
            assert!(issues.iter().any(|issue| {
                issue.location == format!("{slot}.lifeform.{entity}.{stat}")
                    && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
            }));
        }
    }
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.location.starts_with("A1.lifeform.204.")
                || issue.location.starts_with("D1.lifeform.401."))
            .count(),
        5
    );
}

#[test]
fn explicit_zero_lifeform_combat_percentages_are_valid_without_inventing_cargo_or_speed() {
    let mut evidence = evidence();
    evidence.participants.get_mut("A1").unwrap().lifeform = Some(BTreeMap::from([(
        204,
        combat_ogame_api::reports::PartialLifeformBonus {
            weapon: Some(0.0),
            shield: Some(0.0),
            armour: Some(0.0),
            cargo: None,
            speed: None,
        },
    )]));

    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence,
        universe: universe(),
    });
    let CompletionResult::Verified { input } = result else {
        panic!("complete explicit zero combat percentages should verify");
    };
    let bonus = input.request.attacker.lifeform.get(204);
    for value in [
        bonus.weapon,
        bonus.shield,
        bonus.armour,
        bonus.cargo,
        bonus.speed,
    ] {
        assert!(value.abs() < f32::EPSILON);
    }
    let evidence = &input.evidence.fields["A1.lifeform.204"].value;
    assert_eq!(
        evidence,
        &serde_json::json!({
            "weapon": 0.0,
            "shield": 0.0,
            "armour": 0.0
        })
    );
}

#[test]
fn negative_lifeform_combat_percentages_are_rejected_per_stat() {
    let mut evidence = evidence();
    evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .lifeform
        .as_mut()
        .unwrap()
        .insert(
            204,
            combat_ogame_api::reports::PartialLifeformBonus {
                weapon: Some(-500.0),
                shield: Some(-500.0),
                armour: Some(-500.0),
                ..Default::default()
            },
        );

    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("negative lifeform combat percentages must not produce a request");
    };
    for stat in ["weapon", "shield", "armour"] {
        assert!(issues.iter().any(|issue| {
            issue.location == format!("A1.lifeform.204.{stat}")
                && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unsupported
        }));
    }
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.location.starts_with("A1.lifeform.204."))
            .count(),
        3
    );
}

#[test]
fn nonfinite_lifeform_combat_percentages_are_rejected_for_library_callers() {
    let mut evidence = evidence();
    evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .lifeform
        .as_mut()
        .unwrap()
        .insert(
            204,
            combat_ogame_api::reports::PartialLifeformBonus {
                weapon: Some(f32::NAN),
                shield: Some(f32::INFINITY),
                armour: Some(f32::NEG_INFINITY),
                cargo: Some(f32::NAN),
                speed: Some(f32::INFINITY),
            },
        );

    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("nonfinite lifeform combat percentages must not produce a request");
    };
    for stat in ["weapon", "shield", "armour", "cargo", "speed"] {
        assert!(issues.iter().any(|issue| {
            issue.location == format!("A1.lifeform.204.{stat}")
                && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unsupported
        }));
    }
}

#[test]
fn finite_lifeform_percentages_that_overflow_starting_stats_are_rejected() {
    let mut evidence = evidence();
    evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .lifeform
        .as_mut()
        .unwrap()
        .insert(
            204,
            combat_ogame_api::reports::PartialLifeformBonus {
                weapon: Some(f32::MAX),
                shield: Some(f32::MAX),
                armour: Some(f32::MAX),
                cargo: Some(f32::MAX),
                speed: Some(f32::MAX),
            },
        );

    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("overflowing starting combat stats must not produce a request");
    };
    for stat in ["weapon", "armour"] {
        assert!(issues.iter().any(|issue| {
            issue.location == format!("A1.lifeform.204.{stat}")
                && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unsupported
        }));
    }
}

#[test]
fn omitted_lifeform_evidence_stays_unknown_even_when_other_evidence_exists() {
    let mut value = serde_json::to_value(evidence()).unwrap();
    let participants = value["participants"].as_object_mut().unwrap();
    for participant in participants.values_mut() {
        participant.as_object_mut().unwrap().remove("lifeform");
    }
    let evidence_without_lifeform: CompletionEvidence = serde_json::from_value(value).unwrap();

    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence_without_lifeform,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("omitted lifeform evidence must not become an empty bonus map");
    };
    for slot in ["A1", "D1"] {
        assert!(issues.iter().any(|issue| {
            issue.location == format!("{slot}.lifeform")
                && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unknown
        }));
    }

    let mut null_value = serde_json::to_value(evidence()).unwrap();
    for participant in null_value["participants"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        participant
            .as_object_mut()
            .unwrap()
            .insert("lifeform".to_owned(), serde_json::Value::Null);
    }
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: serde_json::from_value(null_value).unwrap(),
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("null lifeform evidence must not become an empty bonus map");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.lifeform"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unknown
    }));
}

#[test]
fn a_pinned_universe_from_another_report_is_rejected() {
    let mut pinned = universe();
    pinned.community = "de".to_owned();
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: pinned,
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("a mismatched universe must not produce a runnable request");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "universe.identity"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::WrongUniverse
    }));
}

#[test]
fn completing_a_private_observation_does_not_bypass_fixture_publication_consent() {
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: universe(),
    });
    let CompletionResult::Verified { input } = result else {
        panic!("the synthetic candidate should be complete");
    };
    let fixture = serde_json::json!({
        "schema_version": 1,
        "name": "private completion result",
        "provenance": {
            "observed_battle": true,
            "source": "local report",
            "universe": "en-1",
            "approximate_date": "2023-11-14",
            "game_version": "13.0.1",
            "publication_consent": false
        },
        "request": input.request,
        "observed": {
            "outcome": "AttackersWin",
            "attacker_losses": {},
            "defender_losses": {},
            "debris": {"metal": 0, "crystal": 0, "deuterium": 0}
        },
        "tolerance": {
            "minimum_observed_outcome_rate": 0.0,
            "losses": {"absolute": 0.0, "relative": 0.0},
            "debris": {"absolute": 0.0, "relative": 0.0},
            "justification": "synthetic consent boundary test"
        }
    });
    let path = std::env::temp_dir().join(format!(
        "completion-private-fixture-{}.json",
        std::process::id()
    ));
    std::fs::write(&path, serde_json::to_vec(&fixture).unwrap()).unwrap();
    let loaded = combat_fixtures::load_fixture(&path).expect("fixture should parse");
    let _ = std::fs::remove_file(&path);
    assert!(
        loaded
            .validation_errors()
            .iter()
            .any(|error| error
                == "observed battles require provenance.publication_consent to be true")
    );
}

#[test]
fn explicitly_known_different_technology_bases_are_reconciled_and_conflicts_rejected() {
    let mut already_effective_report = candidate(Some(204), Some(401));
    already_effective_report.attackers[0].technology = TechnologyCandidate {
        basis: "already_effective".to_owned(),
        weapon: Some(1),
        shield: Some(1),
        armour: Some(1),
    };
    let mut researched_evidence = evidence();
    researched_evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .technology = Some(TechnologyEvidence {
        basis: TechnologyBasis::Researched,
        weapon: 10,
        shield: 10,
        armour: 10,
    });
    let result = complete_candidate(&CompletionInput {
        candidate: already_effective_report,
        evidence: researched_evidence,
        universe: universe(),
    });
    assert!(matches!(result, CompletionResult::Incomplete { ref issues }
        if issues.iter().any(|issue| issue.kind == combat_ogame_api::reports::FieldIssueKind::Contradictory)));

    let mut researched_report = candidate(Some(204), Some(401));
    researched_report.attackers[0].technology = TechnologyCandidate {
        basis: "researched".to_owned(),
        weapon: Some(100),
        shield: Some(100),
        armour: Some(100),
    };
    let mut already_effective_evidence = evidence();
    already_effective_evidence
        .participants
        .get_mut("A1")
        .unwrap()
        .technology = Some(TechnologyEvidence {
        basis: TechnologyBasis::AlreadyEffective,
        weapon: 13,
        shield: 13,
        armour: 13,
    });
    let result = complete_candidate(&CompletionInput {
        candidate: researched_report,
        evidence: already_effective_evidence,
        universe: universe(),
    });
    assert!(matches!(result, CompletionResult::Incomplete { ref issues }
        if issues.iter().any(|issue| issue.kind == combat_ogame_api::reports::FieldIssueKind::Contradictory)));
}

#[test]
fn engine_starting_stats_accept_weapon_rounding_and_armour_hull_units() {
    let mut attacker = participant("A1", Some(204));
    attacker.reported_unit_stats = Some(serde_json::json!({
        "204": {"weapon": 115, "shield": 23, "armor": 920}
    }));
    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers = vec![attacker];
    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence: evidence(),
        universe: universe(),
    });
    assert!(matches!(result, CompletionResult::Verified { .. }));
}

#[test]
fn missing_classes_and_unsupported_compositions_block_a_request() {
    let mut attacker = participant("A1", Some(204));
    attacker.character_class_id = None;
    attacker.alliance_class_id = None;
    let mut candidate = candidate(Some(204), Some(401));
    candidate.attackers = vec![attacker];
    let mut evidence = evidence();
    evidence.participants.get_mut("A1").unwrap().player_class = None;
    evidence.participants.get_mut("A1").unwrap().alliance_class = None;
    evidence.participants.get_mut("A1").unwrap().entities = Some(BTreeMap::from([(9999, 20)]));
    let result = complete_candidate(&CompletionInput {
        candidate,
        evidence,
        universe: universe(),
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("expected incomplete result");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "A1.entities.9999"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unsupported
    }));
    assert!(
        issues
            .iter()
            .any(|issue| issue.location == "A1.player_class")
    );
    assert!(
        issues
            .iter()
            .any(|issue| issue.location == "A1.alliance_class")
    );
}

#[test]
fn every_missing_universe_setting_has_a_targeted_issue() {
    let mut pinned = universe();
    pinned.settings = PinnedUniverseSettings::default();
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: pinned,
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("expected incomplete result");
    };
    for field in [
        "galaxies",
        "systems",
        "donut_galaxy",
        "donut_systems",
        "fleet_speed",
        "rapid_fire",
        "deuterium_save_factor",
    ] {
        assert!(
            issues
                .iter()
                .any(|issue| issue.location == format!("universe.settings.{field}")),
            "missing targeted issue for {field}"
        );
    }
}

#[test]
fn missing_and_invalid_universe_settings_are_reported_together() {
    let mut pinned = universe();
    pinned.settings.galaxies = Some(0);
    pinned.settings.systems = None;
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: pinned,
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("expected incomplete result");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "universe.settings.galaxies"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Unsupported
    }));
    assert!(issues.iter().any(|issue| {
        issue.location == "universe.settings.systems"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
    }));
}

#[test]
fn supplied_universe_and_battle_provenance_remain_distinct() {
    let mut pinned = universe();
    pinned.source = EvidenceSource::Supplied;
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: pinned,
    });
    let CompletionResult::Verified { input } = result else {
        panic!("expected verified result");
    };
    assert_eq!(
        input.evidence.fields["universe"].source,
        EvidenceSource::Supplied
    );
    assert_eq!(
        input.evidence.fields["battle.provenance"].source,
        EvidenceSource::Report
    );
    assert_eq!(
        input.evidence.fields["loot_percentage"].source,
        EvidenceSource::Report
    );
}

#[test]
fn omitted_temporal_status_is_a_missing_issue() {
    let mut value = serde_json::to_value(universe()).unwrap();
    value.as_object_mut().unwrap().remove("current");
    value
        .as_object_mut()
        .unwrap()
        .remove("acknowledged_current");
    let pinned: PinnedUniverse = serde_json::from_value(value).unwrap();
    let result = complete_candidate(&CompletionInput {
        candidate: candidate(Some(204), Some(401)),
        evidence: evidence(),
        universe: pinned,
    });
    let CompletionResult::Incomplete { issues } = result else {
        panic!("expected incomplete result");
    };
    assert!(issues.iter().any(|issue| {
        issue.location == "universe.current"
            && issue.kind == combat_ogame_api::reports::FieldIssueKind::Missing
    }));
}
