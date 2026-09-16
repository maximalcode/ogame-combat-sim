use combat_ogame_api::reports::{
    Candidate, CompletionEvidence, EvidenceSource, Participant, ParticipantEvidence,
    PinnedUniverse, PinnedUniverseSettings, Provenance, TechnologyBasis, TechnologyCandidate,
    TechnologyEvidence,
};
use combat_types::{AllianceClass, PlayerClass};
use std::collections::BTreeMap;

pub fn universe() -> PinnedUniverse {
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

pub fn participant(slot: &str, entity: Option<u16>) -> Participant {
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

pub fn candidate(attacker: Option<u16>, defender: Option<u16>) -> Candidate {
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

pub fn evidence() -> CompletionEvidence {
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
        historical_rapid_fire: None,
    }
}
