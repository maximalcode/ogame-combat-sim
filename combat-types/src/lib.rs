pub mod combat_report;
pub mod entities;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export combat report types for convenience
pub use combat_report::{
    BattleType, CombatReport, EconomicSummary, FleetSnapshot, HarvestInfo, MoonDestructionInfo,
    Participant, ResourceCost, RoundDetails, classify_battle_type,
};

/// Entity type identifier (202-219 for ships, 401-408 for defenses, 502-503 for missiles)
pub type EntityType = u16;

/// Base statistics for an entity type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStats {
    pub entity_type: EntityType,
    pub weapon: u32,
    pub shield: u32,
    pub armour: u32,
    pub rapid_fire_from: HashMap<EntityType, u16>,
    pub rapid_fire_against: HashMap<EntityType, u16>,
    // Economic data
    pub cost_metal: u32,
    pub cost_crystal: u32,
    pub cost_deuterium: u32,
    pub cargo_capacity: u32,
    pub base_speed: u32,
    pub fuel_consumption: u32,
}

/// Technology levels for a player
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Technology {
    #[serde(default)]
    pub weapon: u8,
    #[serde(default)]
    pub shield: u8,
    #[serde(default)]
    pub armour: u8,
    // Drive technologies (Phase C)
    #[serde(default)]
    pub combustion: Option<u8>,
    #[serde(default)]
    pub impulse: Option<u8>,
    #[serde(default)]
    pub hyperspace: Option<u8>,
    #[serde(default)]
    pub hyperspace_tech: Option<u8>,
}

/// Universe-specific settings that affect combat calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseSettings {
    #[serde(default = "default_galaxies")]
    pub galaxies: u8,
    #[serde(default = "default_systems")]
    pub systems: u16,
    #[serde(default = "default_true")]
    pub donut_galaxy: bool,
    #[serde(default = "default_true")]
    pub donut_systems: bool,
    #[serde(default = "default_one")]
    pub fleet_speed: u8,
    #[serde(default = "default_debris_fleet")]
    pub debris_fleet: u8,
    #[serde(default)]
    pub debris_defence: u8,
    #[serde(default)]
    pub deuterium_save_factor: u8,
}

fn default_galaxies() -> u8 {
    9
}
fn default_systems() -> u16 {
    499
}
fn default_true() -> bool {
    true
}
fn default_one() -> u8 {
    1
}
fn default_debris_fleet() -> u8 {
    30
}

impl Default for UniverseSettings {
    fn default() -> Self {
        Self {
            galaxies: 9,
            systems: 499,
            donut_galaxy: true,
            donut_systems: true,
            fleet_speed: 1,
            debris_fleet: 30,
            debris_defence: 0,
            deuterium_save_factor: 0,
        }
    }
}

/// Player class type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PlayerClass {
    #[default]
    None,
    Collector,
    General,
    Discoverer,
}

/// Alliance class type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AllianceClass {
    #[default]
    None,
    Trader,
    Warrior,
    Researcher,
}

/// Player bonuses from classes, officers, and lifeforms
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerBonuses {
    #[serde(default)]
    pub player_class: PlayerClass,
    #[serde(default)]
    pub alliance_class: AllianceClass,
    #[serde(default)]
    pub has_engineer: bool,
    #[serde(default)]
    pub lifeform_bonus: u8, // percentage
}

/// Fleet composition: entity type -> count
pub type FleetComposition = HashMap<EntityType, u32>;

/// Combat party (attacker or defender)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyData {
    pub technology: Technology,
    pub entities: FleetComposition,
}

/// Result summary for an individual slot (attacker or defender)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotResult {
    /// Slot identifier (e.g., "A1", "D2")
    pub slot_id: String,
    /// Initial fleet in this slot
    pub initial: FleetComposition,
    /// Ships lost in this slot
    pub losses: FleetComposition,
    /// Remaining ships in this slot
    pub remaining: FleetComposition,
}

/// A named combat slot (e.g., "A1", "D2") keeping party data separate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartySlot {
    /// Slot identifier (e.g., "A1", "A2", "D1")
    pub id: String,
    /// Optional display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Party data for this slot
    pub data: PartyData,
}

/// Planet resources (for loot calculation)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanetResources {
    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,
}

impl PlanetResources {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.metal + self.crystal + self.deuterium
    }
}

/// Debris field from destroyed ships
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DebrisField {
    pub metal: u64,
    pub crystal: u64,
}

impl DebrisField {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.metal + self.crystal
    }
}

/// Combat simulation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatRequest {
    pub attacker: PartyData,
    pub defender: PartyData,
    /// Optional multi-slot attackers (if provided, engine should respect slots)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_slots: Option<Vec<PartySlot>>,
    /// Optional multi-slot defenders (if provided, engine should respect slots)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_slots: Option<Vec<PartySlot>>,
    #[serde(default)]
    pub planet_resources: Option<PlanetResources>,
    #[serde(default = "default_debris_percentage")]
    pub debris_percentage: f32,
    pub use_rapid_fire: bool,
    pub simulations: u32,
    #[serde(default)]
    pub enable_downscaling: Option<bool>, // None = auto, Some(true) = force on, Some(false) = force off
    /// Optional: compute and return per-round, per-ship-type composition snapshots
    #[serde(default)]
    pub enable_round_compositions: Option<bool>,
    /// Universe settings (debris %, fleet speed, etc.)
    #[serde(default)]
    pub universe_settings: Option<UniverseSettings>,
    /// Attacker bonuses (class, engineer, lifeform)
    #[serde(default)]
    pub attacker_bonuses: Option<PlayerBonuses>,
    /// Defender bonuses (class, engineer, lifeform)
    #[serde(default)]
    pub defender_bonuses: Option<PlayerBonuses>,
    /// Plunder percentage (50, 75, or 100)
    #[serde(default = "default_plunder_percentage")]
    pub plunder_percentage: u8,
}

fn default_debris_percentage() -> f32 {
    30.0 // 30% default
}

fn default_plunder_percentage() -> u8 {
    50 // 50% default
}

/// Combat outcome for a single simulation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CombatOutcome {
    AttackersWin,
    DefendersWin,
    Draw,
}

/// Results from a single simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub outcome: CombatOutcome,
    pub rounds: u8,
    pub attacker_losses: FleetComposition,
    pub defender_losses: FleetComposition,
    pub attacker_remaining: FleetComposition,
    pub defender_remaining: FleetComposition,
    // Economic data
    pub debris_field: DebrisField,
    pub loot: PlanetResources,
    pub attacker_profit: i64,
    pub defender_profit: i64,
    /// Optional detailed per-round information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_details: Option<Vec<RoundDetails>>,
    /// Optional per-round, per-ship-type composition snapshots (aggregated by side)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_compositions: Option<Vec<RoundComposition>>,
    /// Optional per-round, per-ship-type composition snapshots by slot id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_compositions_by_slot: Option<HashMap<String, Vec<RoundComposition>>>,
    /// Optional per-slot results when multi-slot combat is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_slots: Option<Vec<SlotResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_slots: Option<Vec<SlotResult>>,
}

/// Per-round composition snapshot for per-round detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundComposition {
    pub round_number: u8,
    pub attacker_by_type_start: FleetComposition,
    pub defender_by_type_start: FleetComposition,
    pub attacker_by_type_destroyed: FleetComposition,
    pub defender_by_type_destroyed: FleetComposition,
    pub attacker_by_type_end: FleetComposition,
    pub defender_by_type_end: FleetComposition,
}

/// Aggregated results from multiple simulations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatResults {
    pub simulations: u32,
    pub attacker_wins: u32,
    pub defender_wins: u32,
    pub draws: u32,
    pub results: Vec<SimulationResult>,
    pub duration_ms: u64,
    pub average_rounds: f64,
}

impl CombatResults {
    #[must_use]
    pub fn new(simulations: u32) -> Self {
        Self {
            simulations,
            attacker_wins: 0,
            defender_wins: 0,
            draws: 0,
            results: Vec::with_capacity(simulations as usize),
            duration_ms: 0,
            average_rounds: 0.0,
        }
    }

    pub fn add_result(&mut self, result: SimulationResult) {
        match result.outcome {
            CombatOutcome::AttackersWin => self.attacker_wins += 1,
            CombatOutcome::DefendersWin => self.defender_wins += 1,
            CombatOutcome::Draw => self.draws += 1,
        }
        self.results.push(result);
    }

    #[must_use]
    pub fn attacker_win_rate(&self) -> f64 {
        f64::from(self.attacker_wins) / f64::from(self.simulations)
    }

    #[must_use]
    pub fn defender_win_rate(&self) -> f64 {
        f64::from(self.defender_wins) / f64::from(self.simulations)
    }

    #[must_use]
    pub fn draw_rate(&self) -> f64 {
        f64::from(self.draws) / f64::from(self.simulations)
    }
}
