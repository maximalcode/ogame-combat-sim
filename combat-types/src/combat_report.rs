use crate::{CombatOutcome, DebrisField, FleetComposition, PlanetResources, Technology};
/// Comprehensive combat report structure for displaying in-game
/// This mirrors what players see in `OGame` combat reports
use serde::{Deserialize, Serialize};

/// Participant in combat (attacker or defender)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    /// Player name (e.g., "Emperor Palpatine")
    pub name: String,

    /// Player ID in your game database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u64>,

    /// Planet/moon coordinates (e.g., "[1:234:5]")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<String>,

    /// The technology levels this participant fought at — effective levels,
    /// so a General's two and a Warrior alliance's one are already in them.
    /// Not necessarily the levels the request asked for; see
    /// [`Technology::effective_levels`].
    pub technology: Technology,

    /// Alliance name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alliance: Option<String>,
}

/// Fleet state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetSnapshot {
    /// Fleet composition: entity type -> count
    pub ships: FleetComposition,

    /// Total fleet value in resources
    pub total_value: ResourceCost,
}

/// Resource cost breakdown
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceCost {
    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,
}

impl ResourceCost {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.metal + self.crystal + self.deuterium
    }

    pub fn add(&mut self, other: &ResourceCost) {
        self.metal += other.metal;
        self.crystal += other.crystal;
        self.deuterium += other.deuterium;
    }
}

/// Round-by-round combat details (optional, for detailed reports)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundDetails {
    pub round_number: u8,

    /// Attackers alive at start of round
    pub attackers_start: u32,

    /// Defenders alive at start of round
    pub defenders_start: u32,

    /// Attackers destroyed this round
    pub attackers_destroyed: u32,

    /// Defenders destroyed this round
    pub defenders_destroyed: u32,

    /// Attackers alive at end of round
    pub attackers_end: u32,

    /// Defenders alive at end of round
    pub defenders_end: u32,

    /// Hull damage dealt by attackers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_damage: Option<u64>,

    /// Hull damage dealt by defenders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_damage: Option<u64>,

    /// Total shots fired by attackers in this round
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_shots: Option<u64>,

    /// Total shots fired by defenders in this round
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_shots: Option<u64>,

    /// Total shield damage absorbed from attacker shots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_shield_damage: Option<u64>,

    /// Total shield damage absorbed from defender shots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defender_shield_damage: Option<u64>,
}

/// Moon destruction chance (if Death Stars involved)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonDestructionInfo {
    /// Chance to destroy the moon (%)
    pub destruction_chance: f64,

    /// Chance for Death Stars to explode (RIP chance, %)
    pub rip_chance: f64,

    /// Number of Death Stars involved
    pub death_stars: u32,

    /// Moon size (diameter in km)
    pub moon_size: u32,
}

/// Economic summary of the battle.
///
/// `attacker_profit` and `defender_profit` are alternative scenarios: each assumes that side
/// harvests the entire debris field. They are not two halves of one ledger, and summing them
/// double-counts the field. Both use [`DebrisField::total`], so defence debris and deuterium
/// debris affect both figures when enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSummary {
    /// Debris field created
    pub debris_field: DebrisField,

    /// Moon creation chance (based on debris size)
    pub moon_chance: f64,

    /// Resources looted by attacker
    pub plunder: PlanetResources,

    /// Attacker's cost of losses
    pub attacker_losses_cost: ResourceCost,

    /// Defender's cost of losses
    pub defender_losses_cost: ResourceCost,

    /// Attacker's net profit, assuming the attacker harvests the entire field
    pub attacker_profit: i64,

    /// Defender's net profit, assuming the defender harvests the entire field
    pub defender_profit: i64,

    /// Harvest info (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harvest_info: Option<HarvestInfo>,
}

/// Recycler harvest information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestInfo {
    /// Recyclers needed to collect all debris
    pub recyclers_needed: u32,

    /// Estimated harvest time (in seconds)
    pub harvest_time_seconds: u32,
}

/// Comprehensive combat report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatReport {
    // === METADATA ===
    /// Unique battle ID
    pub battle_id: String,

    /// Timestamp of the battle (Unix timestamp)
    pub timestamp: u64,

    /// Battle type (e.g., "Fleet vs Fleet", "Fleet vs Defense", "IPM Attack")
    pub battle_type: BattleType,

    // === PARTICIPANTS ===
    pub attacker: Participant,
    pub defender: Participant,

    // === BATTLE OUTCOME ===
    pub outcome: CombatOutcome,
    pub rounds: u8,

    // === FLEET STATES ===
    /// Attacker's fleet before battle
    pub attacker_fleet_start: FleetSnapshot,

    /// Defender's fleet before battle
    pub defender_fleet_start: FleetSnapshot,

    /// Attacker's losses
    pub attacker_losses: FleetSnapshot,

    /// Defender's losses
    pub defender_losses: FleetSnapshot,

    /// Attacker's remaining fleet
    pub attacker_fleet_end: FleetSnapshot,

    /// Defender's remaining fleet
    pub defender_fleet_end: FleetSnapshot,

    // === ECONOMICS ===
    pub economics: EconomicSummary,

    // === DETAILED BREAKDOWN (OPTIONAL) ===
    /// Round-by-round details (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_details: Option<Vec<RoundDetails>>,

    /// Moon destruction info (if Death Stars involved)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moon_destruction: Option<MoonDestructionInfo>,

    // === METADATA ===
    /// Simulation count (how many sims were run)
    pub simulation_count: u32,

    /// Duration of simulation in milliseconds
    pub duration_ms: u64,
}

/// Battle type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BattleType {
    /// Fleet vs Fleet (normal combat)
    FleetVsFleet,

    /// Fleet vs Defense
    FleetVsDefense,

    /// Mixed (ships and defenses on both sides)
    Mixed,

    /// IPM attack (missile vs defense)
    MissileAttack,

    /// Moon destruction attempt
    MoonDestruction,
}

impl CombatReport {
    /// Create a report ID from timestamp and participants
    #[must_use]
    pub fn generate_battle_id(
        timestamp: u64,
        attacker_id: Option<u64>,
        defender_id: Option<u64>,
    ) -> String {
        format!(
            "cr-{}-{}-{}",
            timestamp,
            attacker_id.unwrap_or(0),
            defender_id.unwrap_or(0)
        )
    }

    /// Calculate moon creation chance based on debris size
    /// `OGame` formula: chance = min(20%, `debris_size` / `100_000`)
    #[must_use]
    pub fn calculate_moon_chance(debris_field: &DebrisField) -> f64 {
        let debris_total = debris_field.total() as f64;

        (debris_total / 100_000.0).min(20.0)
    }

    /// Calculate recyclers needed for debris collection
    /// Each recycler has 20k cargo capacity
    #[must_use]
    pub fn calculate_recyclers_needed(debris_field: &DebrisField) -> u32 {
        const RECYCLER_CAPACITY: u64 = 20_000;
        let total_debris = debris_field.total();
        total_debris.div_ceil(RECYCLER_CAPACITY) as u32
    }

    /// Estimate harvest time based on debris and distance
    /// Simplified formula - you can adjust based on your game mechanics
    #[must_use]
    pub fn estimate_harvest_time(recyclers: u32, debris_field: &DebrisField) -> u32 {
        const SECONDS_PER_TRIP: u32 = 60; // Placeholder - adjust based on distance
        if recyclers == 0 {
            return 0;
        }
        let total_debris = debris_field.total();
        let capacity = u64::from(recyclers) * 20_000;
        let trips = total_debris.div_ceil(capacity).max(1);
        trips as u32 * SECONDS_PER_TRIP
    }
}

/// Helper function to classify battle type
#[must_use]
pub fn classify_battle_type(
    attacker_ships: &FleetComposition,
    defender_ships: &FleetComposition,
) -> BattleType {
    let attacker_has_ships = attacker_ships.keys().any(|&t| (202..=219).contains(&t));
    let attacker_has_defense = attacker_ships.keys().any(|&t| (401..=408).contains(&t));
    let defender_has_ships = defender_ships.keys().any(|&t| (202..=219).contains(&t));
    let defender_has_defense = defender_ships.keys().any(|&t| (401..=408).contains(&t));

    if attacker_has_ships && defender_has_ships && !attacker_has_defense && !defender_has_defense {
        BattleType::FleetVsFleet
    } else if attacker_has_ships && !defender_has_ships && defender_has_defense {
        BattleType::FleetVsDefense
    } else {
        BattleType::Mixed
    }
}
