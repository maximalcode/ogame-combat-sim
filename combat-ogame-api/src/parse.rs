use serde::de::DeserializeOwned;

use crate::{Error, Highscore, PlayerData, Players, ServerData, UniverseData};

fn parse<T: DeserializeOwned>(endpoint: &'static str, xml: &str) -> Result<T, Error> {
    quick_xml::de::from_str(xml).map_err(|source| Error::Parse { endpoint, source })
}

/// Deserialize `serverData.xml` without performing network I/O.
pub fn parse_server_data(xml: &str) -> Result<ServerData, Error> {
    parse("serverData.xml", xml)
}

/// Deserialize `players.xml` without performing network I/O.
pub fn parse_players(xml: &str) -> Result<Players, Error> {
    parse("players.xml", xml)
}

/// Deserialize `universe.xml` without performing network I/O.
pub fn parse_universe(xml: &str) -> Result<UniverseData, Error> {
    parse("universe.xml", xml)
}

/// Deserialize `playerData.xml` without performing network I/O.
pub fn parse_player_data(xml: &str) -> Result<PlayerData, Error> {
    parse("playerData.xml", xml)
}

/// Deserialize `highscore.xml` without performing network I/O.
pub fn parse_highscore(xml: &str) -> Result<Highscore, Error> {
    parse("highscore.xml", xml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_xml_names_the_endpoint() {
        let error = parse_players("<players><player></players>")
            .expect_err("malformed players XML should fail");

        assert!(error.to_string().contains("players.xml"));
    }
}
