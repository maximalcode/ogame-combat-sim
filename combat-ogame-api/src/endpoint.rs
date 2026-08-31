use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use crate::Error;

const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const WEEK: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// A validated `OGame` server name, such as `s1-en`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Universe(String);

impl Universe {
    /// Validate a per-universe host prefix.
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        let Some((number, community)) = value
            .strip_prefix('s')
            .and_then(|rest| rest.split_once('-'))
        else {
            return Err(Error::InvalidUniverse(value));
        };
        let valid_number = !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
            && !number.starts_with('0');
        let valid_community = (2..=3).contains(&community.len())
            && community.bytes().all(|byte| byte.is_ascii_lowercase());
        if !valid_number || !valid_community {
            return Err(Error::InvalidUniverse(value));
        }
        Ok(Self(value))
    }

    /// The validated host prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn host(&self) -> String {
        format!("{}.ogame.gameforge.com", self.0)
    }
}

impl fmt::Display for Universe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for Universe {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Highscore category accepted by `highscore.xml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HighscoreCategory(u8);

impl HighscoreCategory {
    pub const PLAYERS: Self = Self(1);
    pub const ALLIANCES: Self = Self(2);

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Highscore type accepted by `highscore.xml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HighscoreType(u8);

impl HighscoreType {
    pub const TOTAL: Self = Self(0);
    pub const ECONOMY: Self = Self(1);
    pub const RESEARCH: Self = Self(2);
    pub const MILITARY: Self = Self(3);
    pub const MILITARY_BUILT: Self = Self(4);
    pub const MILITARY_DESTROYED: Self = Self(5);
    pub const MILITARY_LOST: Self = Self(6);
    pub const HONOUR: Self = Self(7);

    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// One public XML endpoint and any parameters that identify its payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endpoint {
    ServerData,
    Players,
    Universe,
    PlayerData {
        player_id: u64,
    },
    Highscore {
        category: HighscoreCategory,
        score_type: HighscoreType,
    },
}

impl Endpoint {
    /// Update cadence published for this endpoint and used as its cache TTL.
    #[must_use]
    pub const fn ttl(self) -> Duration {
        match self {
            Self::Highscore { .. } => HOUR,
            Self::ServerData | Self::Players => DAY,
            Self::Universe | Self::PlayerData { .. } => WEEK,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ServerData => "serverData.xml",
            Self::Players => "players.xml",
            Self::Universe => "universe.xml",
            Self::PlayerData { .. } => "playerData.xml",
            Self::Highscore { .. } => "highscore.xml",
        }
    }

    pub(crate) fn relative_url(self) -> String {
        match self {
            Self::ServerData => "api/serverData.xml".to_owned(),
            Self::Players => "api/players.xml".to_owned(),
            Self::Universe => "api/universe.xml".to_owned(),
            Self::PlayerData { player_id } => {
                format!("api/playerData.xml?id={player_id}")
            }
            Self::Highscore {
                category,
                score_type,
            } => format!(
                "api/highscore.xml?category={}&type={}",
                category.value(),
                score_type.value()
            ),
        }
    }

    pub(crate) fn cache_file_name(self) -> String {
        match self {
            Self::ServerData => "serverData.xml".to_owned(),
            Self::Players => "players.xml".to_owned(),
            Self::Universe => "universe.xml".to_owned(),
            Self::PlayerData { player_id } => format!("playerData-{player_id}.xml"),
            Self::Highscore {
                category,
                score_type,
            } => format!("highscore-{}-{}.xml", category.value(), score_type.value()),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::PlayerData { player_id } => {
                write!(formatter, "playerData.xml?id={player_id}")
            }
            Self::Highscore {
                category,
                score_type,
            } => write!(
                formatter,
                "highscore.xml?category={}&type={}",
                category.value(),
                score_type.value()
            ),
            endpoint => formatter.write_str(endpoint.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universe_names_cannot_escape_the_gameforge_host() {
        assert!(Universe::new("s1-en").is_ok());
        assert!(Universe::new("s123-pt").is_ok());
        for invalid in ["s0-en", "s1-EN", "s1-en.example.com", "../s1-en", "en1"] {
            assert!(Universe::new(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn endpoint_ttls_follow_their_published_cadence() {
        assert_eq!(Endpoint::ServerData.ttl(), DAY);
        assert_eq!(Endpoint::Players.ttl(), DAY);
        assert_eq!(Endpoint::Universe.ttl(), WEEK);
        assert_eq!(Endpoint::PlayerData { player_id: 1 }.ttl(), WEEK);
        assert_eq!(
            Endpoint::Highscore {
                category: HighscoreCategory::PLAYERS,
                score_type: HighscoreType::TOTAL,
            }
            .ttl(),
            HOUR
        );
    }
}
