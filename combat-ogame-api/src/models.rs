use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::Timestamped;

/// Server settings and per-universe lifeform configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "these independent flags mirror serverData.xml's public schema"
)]
pub struct ServerData {
    #[serde(rename = "@timestamp")]
    pub timestamp: u64,
    #[serde(rename = "@serverId")]
    pub server_id: String,
    pub number: u32,
    pub language: String,
    pub timezone: String,
    pub timezone_offset: String,
    pub domain: String,
    pub version: String,
    pub speed: u32,
    pub speed_fleet_peaceful: u32,
    pub speed_fleet_war: u32,
    pub speed_fleet_holding: u32,
    pub galaxies: u8,
    pub systems: u16,
    #[serde(deserialize_with = "deserialize_xml_bool")]
    pub acs: bool,
    #[serde(deserialize_with = "deserialize_xml_bool")]
    pub rapid_fire: bool,
    #[serde(rename = "defToTF", deserialize_with = "deserialize_xml_bool")]
    pub defence_to_debris: bool,
    pub debris_factor: f64,
    pub debris_factor_def: f64,
    pub repair_factor: f64,
    #[serde(deserialize_with = "deserialize_xml_bool")]
    pub donut_galaxy: bool,
    #[serde(deserialize_with = "deserialize_xml_bool")]
    pub donut_system: bool,
    pub global_deuterium_save_factor: f64,
    #[serde(default, deserialize_with = "deserialize_xml_bool")]
    pub deuterium_in_debris: bool,
    pub lifeform_settings: LifeformSettings,
}

impl Timestamped for ServerData {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Player directory for one universe.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Players {
    #[serde(rename = "@timestamp")]
    pub timestamp: u64,
    #[serde(rename = "@serverId")]
    pub server_id: String,
    #[serde(rename = "player", default)]
    pub players: Vec<Player>,
}

impl Timestamped for Players {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// One player from `players.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Player {
    #[serde(rename = "@id")]
    pub id: u64,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@status", default)]
    pub status: Option<String>,
    #[serde(rename = "@alliance", default)]
    pub alliance_id: Option<u64>,
}

/// Planet and moon directory for one universe.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UniverseData {
    #[serde(rename = "@timestamp")]
    pub timestamp: u64,
    #[serde(rename = "@serverId")]
    pub server_id: String,
    #[serde(rename = "planet", default)]
    pub planets: Vec<Planet>,
}

impl Timestamped for UniverseData {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// One planet from `universe.xml` or `playerData.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Planet {
    #[serde(rename = "@id")]
    pub id: u64,
    #[serde(rename = "@player", default)]
    pub player_id: Option<u64>,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@coords")]
    pub coordinates: String,
    #[serde(rename = "moon", default)]
    pub moon: Option<Moon>,
}

/// One moon nested below a planet.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Moon {
    #[serde(rename = "@id")]
    pub id: u64,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@size")]
    pub size: u32,
}

/// Scores and planets for one player.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerData {
    #[serde(rename = "@timestamp")]
    pub timestamp: u64,
    #[serde(rename = "@serverId")]
    pub server_id: String,
    #[serde(rename = "@id")]
    pub id: u64,
    #[serde(rename = "@name")]
    pub name: String,
    pub positions: PlayerPositions,
    pub planets: PlayerPlanets,
}

impl Timestamped for PlayerData {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// Wrapper used by the XML's `<positions>` element.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerPositions {
    #[serde(rename = "position", default)]
    pub positions: Vec<PlayerPosition>,
}

/// One score category from `playerData.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerPosition {
    #[serde(rename = "@type")]
    pub score_type: u8,
    #[serde(
        rename = "@score",
        default,
        deserialize_with = "deserialize_optional_u64"
    )]
    pub score: Option<u64>,
    #[serde(
        rename = "@ships",
        default,
        deserialize_with = "deserialize_optional_u64"
    )]
    pub ships: Option<u64>,
}

/// Wrapper used by the XML's `<planets>` element.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerPlanets {
    #[serde(rename = "planet", default)]
    pub planets: Vec<Planet>,
}

/// One highscore category and type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Highscore {
    #[serde(rename = "@timestamp")]
    pub timestamp: u64,
    #[serde(rename = "@serverId")]
    pub server_id: String,
    #[serde(rename = "@category")]
    pub category: u8,
    #[serde(rename = "@type")]
    pub score_type: u8,
    #[serde(rename = "player", default)]
    pub players: Vec<HighscoreEntry>,
    #[serde(rename = "alliance", default)]
    pub alliances: Vec<HighscoreEntry>,
}

impl Timestamped for Highscore {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

/// One player or alliance row from `highscore.xml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HighscoreEntry {
    #[serde(rename = "@position")]
    pub position: u32,
    #[serde(rename = "@id")]
    pub id: u64,
    #[serde(rename = "@score")]
    pub score: u64,
    #[serde(
        rename = "@ships",
        default,
        deserialize_with = "deserialize_optional_u64"
    )]
    pub ships: Option<u64>,
}

/// Complete `<lifeformSettings>` section from `serverData.xml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LifeformSettings {
    #[serde(rename = "lifeform", default)]
    pub lifeforms: Vec<LifeformDefinition>,
}

/// Buildings and researches belonging to one lifeform species.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LifeformDefinition {
    #[serde(rename = "@lifeformId")]
    pub lifeform_id: u8,
    pub buildings: LifeformBuildings,
    pub researches: LifeformResearches,
}

/// Wrapper used by the XML's `<buildings>` element.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LifeformBuildings {
    #[serde(rename = "building", default)]
    pub buildings: Vec<LifeformEntry>,
}

/// Wrapper used by the XML's `<researches>` element.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LifeformResearches {
    #[serde(rename = "research", default)]
    pub researches: Vec<LifeformEntry>,
}

/// One lifeform building or research, including every numeric factor it names.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LifeformEntry {
    #[serde(rename = "@technologyId")]
    pub technology_id: u16,
    #[serde(rename = "type")]
    pub effect_type: String,
    pub factors: FactorSet,
}

/// Dynamic numeric factors from one lifeform entry.
///
/// Gameforge uses element names as keys (`growthBase`, `growthFactor`,
/// `technologyMax`, and so on). Keeping those keys preserves the full block as
/// it evolves while still parsing every value as a number. Per-target factor
/// groups are carried separately in `technologies`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FactorSet {
    pub values: BTreeMap<String, f64>,
    pub technologies: Vec<TechnologyFactors>,
}

impl FactorSet {
    /// A direct factor such as `growthFactor` or `costReductionMax`.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }
}

/// One nested `<technology>` target and all of its numeric base/factor/max values.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TechnologyFactors {
    pub values: BTreeMap<String, f64>,
}

impl TechnologyFactors {
    /// A target factor such as `weaponBase`, `weaponFactor` or `technologyMax`.
    #[must_use]
    pub fn value(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }

    /// Entity id named by `techId`, when this factor group targets an entity.
    #[must_use]
    pub fn entity_id(&self) -> Option<u16> {
        let value = self.value("techId")?;
        if value.fract() == 0.0 && (0.0..=f64::from(u16::MAX)).contains(&value) {
            Some(value as u16)
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for FactorSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FactorSetVisitor;

        impl<'de> Visitor<'de> for FactorSetVisitor {
            type Value = FactorSet;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("numeric lifeform factors")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut factors = FactorSet::default();
                while let Some(name) = map.next_key::<String>()? {
                    if name == "technology" {
                        factors.technologies.push(map.next_value()?);
                    } else {
                        let value = map.next_value::<f64>()?;
                        factors.values.insert(name, value);
                    }
                }
                Ok(factors)
            }
        }

        deserializer.deserialize_map(FactorSetVisitor)
    }
}

impl<'de> Deserialize<'de> for TechnologyFactors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TechnologyFactorsVisitor;

        impl<'de> Visitor<'de> for TechnologyFactorsVisitor {
            type Value = TechnologyFactors;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("numeric factors for one lifeform target")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();
                while let Some(name) = map.next_key::<String>()? {
                    values.insert(name, map.next_value::<f64>()?);
                }
                Ok(TechnologyFactors { values })
            }
        }

        deserializer.deserialize_map(TechnologyFactorsVisitor)
    }
}

fn deserialize_xml_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    match value.trim() {
        "" | "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        other => Err(de::Error::custom(format_args!(
            "expected XML boolean, got {other:?}"
        ))),
    }
}

fn deserialize_optional_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        value.parse().map(Some).map_err(de::Error::custom)
    }
}
