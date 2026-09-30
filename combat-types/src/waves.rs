//! Separate attack missions against one target, with exact per-run survivors.
use crate::{
    DebrisField, DebrisSettings, EntityType, PartyData, PlanetResources, PlayerBonuses,
    SimulationResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One mission's fresh attacker. Survivors return after this mission.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttackWave {
    pub attacker: PartyData,
    #[serde(default)]
    pub attacker_bonuses: Option<PlayerBonuses>,
}

/// Engine-only sequence of single-party battles. No wave cap or defence rebuild.
/// All battles run at full size; there is no downscaling between missions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveSequenceRequest {
    pub waves: Vec<AttackWave>,
    pub defender: PartyData,
    #[serde(default)]
    pub defender_bonuses: Option<PlayerBonuses>,
    #[serde(default)]
    pub planet_resources: Option<PlanetResources>,
    #[serde(default)]
    pub debris_settings: DebrisSettings,
    pub use_rapid_fire: bool,
    #[serde(default)]
    pub collect_compositions: bool,
}

/// Sums of mission results, not sums of the target's successive snapshots.
/// Loss counts are widened because fresh attackers may repeat across many waves.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WaveSequenceTotals {
    pub rounds: u64,
    pub attacker_losses: HashMap<EntityType, u64>,
    pub defender_losses: HashMap<EntityType, u64>,
    pub debris_field: DebrisField,
    pub loot: PlanetResources,
    /// Assumes the attacker harvests all debris; alternative to defender profit.
    pub attacker_profit: i64,
    /// Assumes the defender harvests all debris; alternative to attacker profit.
    pub defender_profit: i64,
}

/// One complete trajectory. An empty sequence leaves the target unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveSequenceResult {
    pub waves: Vec<SimulationResult>,
    pub totals: WaveSequenceTotals,
    /// Research and modifiers are retained; only the composition changes.
    pub final_defender: PartyData,
    pub remaining_resources: Option<PlanetResources>,
}
