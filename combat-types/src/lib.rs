pub mod combat_report;
pub mod entities;
pub mod lifeforms;
pub mod names;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export combat report types for convenience
pub use combat_report::{
    BattleType, CombatReport, EconomicSummary, FleetSnapshot, HarvestInfo, MoonDestructionInfo,
    Participant, ResourceCost, RoundDetails, classify_battle_type,
};
pub use lifeforms::{
    BuiltinLifeformTechs, LifeformBonus, LifeformBonuses, LifeformTech, LifeformTechId,
    LifeformTechTable,
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
    /// Percentage of destroyed *ships* that becomes debris. Per-universe in the
    /// live game, 30–80%, 30% in a standard universe.
    #[serde(default = "default_debris_fleet")]
    pub debris_fleet: u8,
    /// Percentage of destroyed *defences* that becomes debris. 0–80%, and 0 —
    /// no defence debris at all — in a standard universe.
    #[serde(default)]
    pub debris_defence: u8,
    /// Whether destroyed hulls also leave their deuterium cost in the debris
    /// field. A per-universe option since v9.2.0-beta1 (Feb 2023); off in a
    /// standard universe, which is why any claim that debris is only ever metal
    /// and crystal reads as true until you meet a universe that enabled it.
    #[serde(default)]
    pub debris_deuterium: bool,
    #[serde(default)]
    pub deuterium_save_factor: u8,
}

/// The debris rules a single battle is resolved under, after
/// [`CombatRequest::debris_settings`] has settled which of the two places they
/// can come from wins.
///
/// A separate type from `UniverseSettings` because it is the part the engine
/// actually consumes: galaxies, systems and fleet speed have nothing to say
/// about a wreck field.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebrisSettings {
    /// Percentage of destroyed ships that becomes debris.
    pub fleet_percentage: u8,
    /// Percentage of destroyed defences that becomes debris.
    pub defence_percentage: u8,
    /// Whether deuterium joins metal and crystal in the field.
    pub deuterium: bool,
}

/// A standard universe, and the same rules the engine applied before universe
/// settings were read: 30% of destroyed ships, no defence debris, no deuterium.
impl Default for DebrisSettings {
    fn default() -> Self {
        Self {
            fleet_percentage: default_debris_fleet(),
            defence_percentage: 0,
            deuterium: false,
        }
    }
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

/// Calls the same functions serde does, for the reason spelled out on
/// `impl Default for CombatRequest`. These values used to be repeated as
/// literals here; they agreed with the serde defaults, but nothing made them.
impl Default for UniverseSettings {
    fn default() -> Self {
        Self {
            galaxies: default_galaxies(),
            systems: default_systems(),
            donut_galaxy: default_true(),
            donut_systems: default_true(),
            fleet_speed: default_one(),
            debris_fleet: default_debris_fleet(),
            debris_defence: 0,
            debris_deuterium: false,
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

/// Effective combat-technology levels a General is worth: +2 to Weapons,
/// Shielding *and* Armour, unchanged since player classes arrived in v7.
const GENERAL_COMBAT_LEVELS: u8 = 2;

/// The Warrior alliance class, +1 to the same three, from v8 (June 2021).
const WARRIOR_COMBAT_LEVELS: u8 = 1;

/// Player bonuses from classes and officers.
///
/// Lifeform research is not here: it is per ship type rather than per player,
/// and rides on [`PartyData::lifeform`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerBonuses {
    #[serde(default)]
    pub player_class: PlayerClass,
    #[serde(default)]
    pub alliance_class: AllianceClass,
    /// The Engineer officer, and **read by nothing**. Its combat effect is on
    /// the rebuild roll after a battle — 70% of destroyed defences come back
    /// free, 85% with an Engineer — and this engine has no rebuild step for it
    /// to modify: a destroyed defence is simply a loss. Wiring it up means
    /// modelling rebuilding first, so the flag round-trips and waits rather
    /// than being turned into a stat bonus it is not.
    #[serde(default)]
    pub has_engineer: bool,
    // A `lifeform_bonus: u8` used to sit here, read by nothing. One flat
    // percentage cannot describe a per-ship-type bonus, so it was widened into
    // `PartyData::lifeform` rather than reinterpreted. A request still sending
    // the old field is ignored, which is what it amounted to before.
}

impl PlayerBonuses {
    /// Levels these bonuses add to **each** of Weapons, Shielding and Armour.
    ///
    /// Player and alliance classes feed one pipeline in the live game —
    /// v13.0.0-beta49 was a fix for their stat bonuses being calculated apart
    /// from each other — so they add, and a General in a Warrior alliance
    /// fights three levels above his research. Three is the ceiling: nothing
    /// else in the game grants combat levels.
    ///
    /// The matches are exhaustive rather than `_ => 0`, so a class added later
    /// has to be classified here instead of silently defaulting to harmless.
    #[must_use]
    pub fn combat_technology_levels(&self) -> u8 {
        // Collector and Discoverer have no combat-stat effect at all;
        // Discoverer's bonus is loot from inactives, which is not a stat.
        let from_player_class = match self.player_class {
            PlayerClass::General => GENERAL_COMBAT_LEVELS,
            PlayerClass::None | PlayerClass::Collector | PlayerClass::Discoverer => 0,
        };
        // Trader and Researcher change trade and research, not battles.
        let from_alliance_class = match self.alliance_class {
            AllianceClass::Warrior => WARRIOR_COMBAT_LEVELS,
            AllianceClass::None | AllianceClass::Trader | AllianceClass::Researcher => 0,
        };

        from_player_class + from_alliance_class
    }
}

impl Technology {
    /// The levels this player actually fights at, once classes are counted.
    ///
    /// Class bonuses are *levels*, not a multiplier applied after the fact: a
    /// General with Weapons 10 fights as Weapons 12, and the `+10%` per level
    /// is applied once, to the total. Every source adds at this stage, so
    /// resolving them here is what keeps the engine unaware that classes
    /// exist — everything downstream sees a `Technology` and cannot tell which
    /// of its levels were researched.
    ///
    /// `None` is the identity, which is what makes a request that names no
    /// bonuses resolve exactly as it did before any of this was read.
    ///
    /// Saturating, so a level-255 party gains nothing rather than wrapping
    /// round to nothing. The drive technologies come through untouched: no
    /// class changes them and no combat code reads them.
    #[must_use]
    pub fn effective_levels(self, bonuses: Option<&PlayerBonuses>) -> Self {
        let extra = bonuses.map_or(0, PlayerBonuses::combat_technology_levels);

        Self {
            weapon: self.weapon.saturating_add(extra),
            shield: self.shield.saturating_add(extra),
            armour: self.armour.saturating_add(extra),
            ..self
        }
    }
}

/// Fleet composition: entity type -> count
pub type FleetComposition = HashMap<EntityType, u32>;

/// Combat party (attacker or defender)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartyData {
    pub technology: Technology,
    pub entities: FleetComposition,
    /// Lifeform research, as percentages added to the base stats of individual
    /// ship types. It lives here rather than in [`PlayerBonuses`] for the same
    /// reason `technology` does: it is a per-side stat modifier the stat cache
    /// reads directly, and keeping the two in one struct is what stops a side
    /// being fought with the other side's numbers. Per *party* rather than per
    /// side is also the truer model — lifeform research is per player, and the
    /// slots of an ACS attack are different players.
    ///
    /// Empty is the identity, and empty is what serde fills in, so a request
    /// that names no lifeforms resolves exactly as it did before they were
    /// modelled.
    #[serde(default, skip_serializing_if = "LifeformBonuses::is_empty")]
    pub lifeform: LifeformBonuses,
}

impl PartyData {
    /// This party fighting at its
    /// [effective levels](Technology::effective_levels).
    ///
    /// The fleet comes through untouched — classes modify stats, never ship
    /// counts. Named for what it returns rather than what it is given: the
    /// bonuses are resolved here and do not survive the call, which is the
    /// whole point — nothing downstream can tell a granted level from a
    /// researched one.
    ///
    /// Lifeform bonuses ride along unchanged. No class scales them, and they
    /// are already resolved percentages by the time a request carries them.
    #[must_use]
    pub fn at_effective_levels(&self, bonuses: Option<&PlayerBonuses>) -> Self {
        Self {
            technology: self.technology.effective_levels(bonuses),
            entities: self.entities.clone(),
            lifeform: self.lifeform.clone(),
        }
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct DebrisField {
    pub metal: u64,
    pub crystal: u64,
    /// Zero unless the universe has deuterium debris switched on — see
    /// [`UniverseSettings::debris_deuterium`]. Defaulted rather than required so
    /// a stored field from before the option existed still deserializes.
    #[serde(default)]
    pub deuterium: u64,
}

impl DebrisField {
    #[must_use]
    pub fn total(&self) -> u64 {
        self.metal + self.crystal + self.deuterium
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
    /// Percentage of destroyed ships that becomes debris, for requests that do
    /// not carry a `universe_settings` block. **When they do, it wins and this
    /// field is ignored** — see [`CombatRequest::debris_settings`].
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

/// Written by hand rather than derived, and the reason is a real bug it avoids.
///
/// Several fields carry serde defaults that are not the type's zero value —
/// `debris_percentage` defaults to 30.0 and `plunder_percentage` to 50. A
/// derived `Default` would set them to `0.0` and `0`, so
/// `CombatRequest { ..Default::default() }` and the equivalent JSON request
/// would quietly produce different debris fields and different loot. Every
/// defaulted field below calls the same function serde calls, so the two
/// cannot drift apart; `default_matches_deserializing_a_minimal_request`
/// asserts it rather than trusting this comment.
///
/// The four fields serde requires — both parties, `use_rapid_fire` and
/// `simulations` — have no serde default to agree with, so the values here are
/// a judgement call: empty fleets, rapid fire on as it is in-game, and a
/// hundred simulations, which is enough for a stable distribution and small
/// enough to be interactive. `simulations` deliberately is not `0`: a request
/// that runs no battles has nothing to average and panics in the report
/// builder.
impl Default for CombatRequest {
    fn default() -> Self {
        Self {
            attacker: PartyData::default(),
            defender: PartyData::default(),
            attacker_slots: None,
            defender_slots: None,
            planet_resources: None,
            debris_percentage: default_debris_percentage(),
            use_rapid_fire: true,
            simulations: 100,
            enable_downscaling: None,
            enable_round_compositions: None,
            universe_settings: None,
            attacker_bonuses: None,
            defender_bonuses: None,
            plunder_percentage: default_plunder_percentage(),
        }
    }
}

impl CombatRequest {
    /// The attacking side as it actually fights: its fleet, at the levels its
    /// classes are worth.
    ///
    /// The pairing of a party with *its own* bonus block lives here rather than
    /// at each call site. Reading `attacker_bonuses` beside `defender` is a bug
    /// no test of either half would catch, and there is more than one consumer
    /// — the simulator fights the battle, the report builder states what it was
    /// fought at. Both go through these two methods so they cannot disagree.
    #[must_use]
    pub fn effective_attacker(&self) -> PartyData {
        self.attacker
            .at_effective_levels(self.attacker_bonuses.as_ref())
    }

    /// The defending side as it actually fights. See
    /// [`effective_attacker`](Self::effective_attacker).
    #[must_use]
    pub fn effective_defender(&self) -> PartyData {
        self.defender
            .at_effective_levels(self.defender_bonuses.as_ref())
    }

    /// Settle which of the two places debris rules can come from wins.
    ///
    /// A request can describe debris twice — once as the top-level
    /// `debris_percentage`, once inside `universe_settings` — and the two can
    /// disagree. **`universe_settings` wins whenever it is present**, because a
    /// caller who has gone to the trouble of describing a universe means it;
    /// `debris_percentage` is the fallback for the requests that do not.
    ///
    /// The fallback deliberately reports no defence debris and no deuterium,
    /// which is what a standard universe does and, not by coincidence, exactly
    /// what this engine computed before universe settings were read at all. A
    /// request with `universe_settings: null` therefore produces the same
    /// wreck field it always did.
    #[must_use]
    pub fn debris_settings(&self) -> DebrisSettings {
        self.universe_settings.as_ref().map_or_else(
            || DebrisSettings {
                // Saturating, so a caller who puts a nonsense figure in an
                // `f32` field gets a clamped percentage rather than a wrapped
                // one. `as` on a float already saturates at the integer bounds
                // in Rust; the clamp pins the upper end at 100 rather than 255.
                fleet_percentage: self.debris_percentage.clamp(0.0, 100.0) as u8,
                defence_percentage: 0,
                deuterium: false,
            },
            |settings| DebrisSettings {
                fleet_percentage: settings.debris_fleet,
                defence_percentage: settings.debris_defence,
                deuterium: settings.debris_deuterium,
            },
        )
    }
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
    /// The debris rules these battles were actually scored under, after
    /// [`CombatRequest::debris_settings`] resolved them. Reported rather than
    /// dropped so a caller can see which of the two sources won without having
    /// to re-derive the rule — a wreck field that disagrees with the game is
    /// far easier to explain when the percentages behind it are visible.
    #[serde(default)]
    pub debris_settings: DebrisSettings,
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
            debris_settings: DebrisSettings::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The property that makes the hand-written `Default` worth having: a
    /// request built with `..Default::default()` and the same request sent as
    /// minimal JSON must be the same request. Comparing the serialised forms
    /// checks every field at once, so a field added later without a matching
    /// `Default` entry fails here rather than silently changing a battle.
    #[test]
    fn default_matches_deserializing_a_minimal_request() {
        // Only the four fields serde genuinely requires. Everything else is
        // left out so serde has to fill it in.
        let from_json: CombatRequest = serde_json::from_str(
            r#"{
                "attacker":  { "technology": {}, "entities": {} },
                "defender":  { "technology": {}, "entities": {} },
                "use_rapid_fire": true,
                "simulations": 100
            }"#,
        )
        .expect("minimal request should deserialize");

        let from_default = CombatRequest {
            use_rapid_fire: true,
            simulations: 100,
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(&from_json).expect("serialize"),
            serde_json::to_value(&from_default).expect("serialize"),
        );
    }

    /// The JSON body is the only route a caller has to lifeform bonuses today —
    /// `POST /api/simulate` and `combat-cli --file` both take this shape — so
    /// the shape is part of the feature and not an implementation detail. Keyed
    /// by entity id, percentages, and every stat optional.
    #[test]
    fn a_party_reads_its_lifeform_bonuses_from_json() {
        let party: PartyData = serde_json::from_str(
            r#"{
                "technology": { "weapon": 25 },
                "entities": { "213": 100 },
                "lifeform": { "213": { "weapon": 50.0 } }
            }"#,
        )
        .expect("a party with lifeform bonuses should deserialize");

        let bonus = party.lifeform.get(213);
        assert!((bonus.weapon - 50.0).abs() < f32::EPSILON);
        assert!(bonus.shield.abs() < f32::EPSILON, "unnamed stays at zero");
        assert_eq!(
            party.lifeform.get(204),
            LifeformBonus::default(),
            "an unnamed ship has no bonus",
        );
    }

    /// A party with no lifeform research must serialize to exactly what it did
    /// before lifeforms were modelled, or every stored request and every API
    /// response gains an empty object.
    #[test]
    fn a_party_with_no_lifeform_research_does_not_mention_it() {
        let json = serde_json::to_value(PartyData::default()).expect("serialize");

        assert!(json.get("lifeform").is_none());
    }

    /// `simulations: 0` has no meaningful reading — it produces no results to
    /// average and panics downstream — so the default must not be zero.
    #[test]
    fn default_runs_a_nonzero_number_of_simulations() {
        assert!(CombatRequest::default().simulations > 0);
    }

    /// The precedence rule, stated as a test because a comment cannot fail.
    /// `universe_settings` describes debris and so does the top-level
    /// `debris_percentage`; when both are present the settings win.
    #[test]
    fn universe_settings_beat_the_top_level_debris_percentage() {
        let request = CombatRequest {
            debris_percentage: 30.0,
            universe_settings: Some(UniverseSettings {
                debris_fleet: 80,
                debris_defence: 50,
                debris_deuterium: true,
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(
            request.debris_settings(),
            DebrisSettings {
                fleet_percentage: 80,
                defence_percentage: 50,
                deuterium: true,
            }
        );
    }

    /// The other half of the rule: with no settings block the top-level field
    /// is the fallback, and it describes fleets only — which is exactly how the
    /// engine behaved before universe settings were wired in.
    #[test]
    fn the_top_level_debris_percentage_is_the_fallback() {
        let request = CombatRequest {
            debris_percentage: 45.0,
            universe_settings: None,
            ..Default::default()
        };

        assert_eq!(
            request.debris_settings(),
            DebrisSettings {
                fleet_percentage: 45,
                defence_percentage: 0,
                deuterium: false,
            }
        );
    }

    /// A universe's debris percentages are whole numbers, but the fallback
    /// field is an `f32` a caller can put anything in. Truncation towards zero
    /// is the safe reading — it never invents debris that is not there — and
    /// the saturating cast keeps `1e9` from wrapping round to a small figure.
    #[test]
    fn a_nonsensical_fallback_percentage_cannot_wrap_around() {
        let huge = CombatRequest {
            debris_percentage: 1e9,
            ..Default::default()
        };
        assert_eq!(huge.debris_settings().fleet_percentage, 100);

        let negative = CombatRequest {
            debris_percentage: -20.0,
            ..Default::default()
        };
        assert_eq!(negative.debris_settings().fleet_percentage, 0);
    }

    #[test]
    fn party_data_defaults_to_an_empty_fleet_at_zero_tech() {
        let party = PartyData::default();
        assert!(party.entities.is_empty());
        assert_eq!(party.technology, Technology::default());
    }

    /// Ten researched levels and a class worth two of them, expressed the way
    /// the rest of the code sees it.
    fn researched(level: u8) -> Technology {
        Technology {
            weapon: level,
            shield: level,
            armour: level,
            ..Default::default()
        }
    }

    fn classes(player_class: PlayerClass, alliance_class: AllianceClass) -> PlayerBonuses {
        PlayerBonuses {
            player_class,
            alliance_class,
            ..Default::default()
        }
    }

    /// The whole point of the word "effective": a General does not multiply
    /// anything, he adds two levels to each of the three combat technologies,
    /// and the `+10%` per level is then applied once to the total.
    #[test]
    fn a_general_is_worth_two_levels_of_each_combat_technology() {
        let bonuses = classes(PlayerClass::General, AllianceClass::None);

        assert_eq!(
            researched(10).effective_levels(Some(&bonuses)),
            researched(12)
        );
    }

    /// The alliance half of the same pipeline, added in v8.
    #[test]
    fn a_warrior_alliance_is_worth_one_level() {
        let bonuses = classes(PlayerClass::None, AllianceClass::Warrior);

        assert_eq!(
            researched(10).effective_levels(Some(&bonuses)),
            researched(11)
        );
    }

    /// Player and alliance classes feed one pipeline in the live game, so they
    /// add rather than override, and +3 is the ceiling — nothing else in the
    /// game grants combat levels.
    #[test]
    fn a_general_in_a_warrior_alliance_stacks_to_three_levels() {
        let bonuses = classes(PlayerClass::General, AllianceClass::Warrior);

        assert_eq!(bonuses.combat_technology_levels(), 3);
        assert_eq!(
            researched(10).effective_levels(Some(&bonuses)),
            researched(13)
        );
    }

    /// Every other class exists and does something, but none of it is a combat
    /// stat: Discoverer takes more loot from inactives, Trader and Researcher
    /// change trade and research. A simulator that quietly gave them levels
    /// would be wrong in a way nobody would notice.
    #[test]
    fn the_other_classes_grant_no_combat_levels() {
        for player_class in [
            PlayerClass::None,
            PlayerClass::Collector,
            PlayerClass::Discoverer,
        ] {
            for alliance_class in [
                AllianceClass::None,
                AllianceClass::Trader,
                AllianceClass::Researcher,
            ] {
                let bonuses = classes(player_class, alliance_class);
                assert_eq!(
                    bonuses.combat_technology_levels(),
                    0,
                    "{player_class:?} + {alliance_class:?}"
                );
            }
        }
    }

    /// A request that names no bonuses must resolve exactly as it did before
    /// any of this was read, so `None` has to be the identity — not "the
    /// default class", which would be the same thing today but is a different
    /// claim.
    #[test]
    fn no_bonuses_leave_the_levels_alone() {
        let tech = researched(10);

        assert_eq!(tech.effective_levels(None), tech);
    }

    /// Levels are `u8`, and a caller is free to send 255. Saturating rather
    /// than wrapping: an absurd level should stay absurd, not become zero.
    #[test]
    fn a_level_at_the_top_of_the_range_cannot_wrap_around() {
        let bonuses = classes(PlayerClass::General, AllianceClass::Warrior);

        assert_eq!(researched(255).effective_levels(Some(&bonuses)), {
            let mut expected = researched(255);
            expected.weapon = u8::MAX;
            expected.shield = u8::MAX;
            expected.armour = u8::MAX;
            expected
        });
    }

    /// Classes change stats, never fleets. Worth pinning because
    /// `at_effective_levels` is the call the simulator makes on every battle.
    #[test]
    fn applying_bonuses_to_a_party_leaves_its_fleet_alone() {
        let party = PartyData {
            technology: researched(10),
            entities: HashMap::from([(204, 100)]),
            ..Default::default()
        };

        let boosted =
            party.at_effective_levels(Some(&classes(PlayerClass::General, AllianceClass::None)));

        assert_eq!(boosted.entities, party.entities);
        assert_eq!(boosted.technology, researched(12));
    }

    /// The pairing is the part worth pinning: each side must be resolved
    /// against *its own* bonus block. Swapping them is a bug that a test of
    /// either side alone would pass straight through.
    #[test]
    fn each_side_resolves_against_its_own_bonuses() {
        let request = CombatRequest {
            attacker: PartyData {
                technology: researched(10),
                entities: HashMap::from([(204, 100)]),
                ..Default::default()
            },
            defender: PartyData {
                technology: researched(4),
                entities: HashMap::from([(401, 50)]),
                ..Default::default()
            },
            attacker_bonuses: Some(classes(PlayerClass::General, AllianceClass::None)),
            defender_bonuses: Some(classes(PlayerClass::None, AllianceClass::Warrior)),
            ..Default::default()
        };

        assert_eq!(request.effective_attacker().technology, researched(12));
        assert_eq!(request.effective_defender().technology, researched(5));
    }
}
