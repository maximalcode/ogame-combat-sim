use std::collections::HashMap;

use approx::assert_abs_diff_eq;
use combat_ogame_api::{
    ServerDataLifeformTechs, parse_highscore, parse_player_data, parse_players, parse_server_data,
    parse_universe,
};
use combat_types::LifeformTechTable;

const SERVER_DATA: &str = include_str!("fixtures/serverData.xml");
const PLAYERS: &str = include_str!("fixtures/players.xml");
const UNIVERSE: &str = include_str!("fixtures/universe.xml");
const PLAYER_DATA: &str = include_str!("fixtures/playerData.xml");
const HIGHSCORE: &str = include_str!("fixtures/highscore.xml");
const ALLIANCE_HIGHSCORE: &str = include_str!("fixtures/highscore-alliances.xml");

#[test]
fn parses_server_settings_and_the_complete_lifeform_factor_shapes() {
    let server = parse_server_data(SERVER_DATA).expect("serverData fixture");

    assert_eq!(server.server_id, "en123");
    assert_eq!(server.speed, 8);
    assert_abs_diff_eq!(server.debris_factor, 0.7);
    assert!(server.deuterium_in_debris);

    let humans = &server.lifeform_settings.lifeforms[0];
    assert_eq!(
        humans.buildings.buildings[0].factors.value("growthFactor"),
        Some(1.2)
    );
    let cost_reduction = &humans.researches.researches[1].factors.technologies[0];
    assert_eq!(cost_reduction.value("technologyBase"), Some(0.1));
    assert_eq!(cost_reduction.value("technologyFactor"), Some(1.0));
    assert_eq!(cost_reduction.value("technologyMax"), Some(0.5));
    assert_eq!(cost_reduction.value("timeTechnologyMax"), Some(0.99));
}

#[test]
fn server_data_is_a_second_lifeform_tech_table_source() {
    let server = parse_server_data(SERVER_DATA).expect("serverData fixture");
    let table = ServerDataLifeformTechs::try_from(&server).expect("lifeform table");

    let fighter = table.tech(11209).expect("light fighter research");
    assert_eq!(fighter.targets, vec![204]);
    assert_abs_diff_eq!(fighter.per_level_percent, 0.3);
    let defences = table.tech(12216).expect("defence research");
    assert_eq!(defences.targets, vec![401, 408]);
    assert_abs_diff_eq!(defences.per_level_percent, 0.5);
    assert!(table.tech(11217).is_none());

    let bonuses = table.resolve(&HashMap::from([(11209, 10)]), 0.0);
    assert_abs_diff_eq!(bonuses.get(204).weapon, 3.0);
}

#[test]
fn parses_player_names_and_alliance_membership() {
    let players = parse_players(PLAYERS).expect("players fixture");

    assert_eq!(players.timestamp, 1_700_000_100);
    assert_eq!(players.players[0].name, "Ada");
    assert_eq!(players.players[0].alliance_id, Some(42));
    assert_eq!(players.players[1].status.as_deref(), Some("vI"));
}

#[test]
fn parses_planets_moons_and_coordinates() {
    let universe = parse_universe(UNIVERSE).expect("universe fixture");

    assert_eq!(universe.planets[0].coordinates, "1:2:3");
    assert_eq!(universe.planets[0].moon.as_ref().expect("moon").size, 8888);
    assert!(universe.planets[1].moon.is_none());
}

#[test]
fn parses_player_scores_and_tolerates_unranked_empty_values() {
    let player = parse_player_data(PLAYER_DATA).expect("playerData fixture");

    assert_eq!(player.name, "Ada");
    assert_eq!(player.positions.positions[1].ships, Some(4321));
    assert_eq!(player.positions.positions[2].score, None);
    assert_eq!(player.planets.planets[0].coordinates, "1:2:3");
}

#[test]
fn parses_highscore_rows_and_optional_ship_counts() {
    let highscore = parse_highscore(HIGHSCORE).expect("highscore fixture");

    assert_eq!(highscore.category, 1);
    assert_eq!(highscore.score_type, 3);
    assert_eq!(highscore.players[0].score, 98765);
    assert_eq!(highscore.players[0].ships, Some(4321));
    assert_eq!(highscore.players[1].ships, None);
}

#[test]
fn parses_alliance_highscore_rows() {
    let highscore = parse_highscore(ALLIANCE_HIGHSCORE).expect("alliance highscore fixture");

    assert_eq!(highscore.category, 2);
    assert!(highscore.players.is_empty());
    assert_eq!(highscore.alliances[0].id, 42);
    assert_eq!(highscore.alliances[0].score, 345_678_901);
}
