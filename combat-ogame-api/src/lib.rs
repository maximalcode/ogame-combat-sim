//! Typed access to `OGame`'s public, per-universe XML metadata endpoints.
//!
//! Fetching, caching and parsing live here rather than in `combat-core`, so a
//! simulation remains fully offline. Parsing is exposed separately and is
//! tested against checked-in XML fixtures; callers that already have XML do
//! not need an HTTP client at all.

mod client;
mod endpoint;
mod error;
mod lifeforms;
mod models;
mod parse;

pub use client::OGameClient;
pub use endpoint::{Endpoint, HighscoreCategory, HighscoreType, Universe};
pub use error::Error;
pub use lifeforms::ServerDataLifeformTechs;
pub use models::{
    FactorSet, Highscore, HighscoreEntry, LifeformBuildings, LifeformDefinition, LifeformEntry,
    LifeformResearches, LifeformSettings, Moon, Planet, Player, PlayerData, PlayerPlanets,
    PlayerPosition, PlayerPositions, Players, ServerData, TechnologyFactors, UniverseData,
};
pub use parse::{
    parse_highscore, parse_player_data, parse_players, parse_server_data, parse_universe,
};

/// A response whose root declares when Gameforge generated it.
pub trait Timestamped {
    /// Unix timestamp from the response root's `timestamp` attribute.
    fn timestamp(&self) -> u64;
}
