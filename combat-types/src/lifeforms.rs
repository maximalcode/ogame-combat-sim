//! Lifeform research, as combat sees it.
//!
//! Lifeforms arrived in `OGame` v9 (June 2022) and are active on every universe
//! sampled, so this is the default condition of the game rather than an edge
//! case. What they grant is a **per-ship-type** bonus that adds to the base
//! stat in the same bracket as technology:
//!
//! ```text
//! attack = floor(baseWeapon * (1 + 0.10*weaponsTech + lifeformBonus))
//! ```
//!
//! Additive, not multiplicative: a Destroyer (base weapon 2000) at Weapons 25
//! with a +50% lifeform bonus comes out at 8000, matching the official forum's
//! worked example. Multiplying would give 10,500.
//!
//! That per-ship-type shape is why lifeform research cannot travel through
//! [`Technology::effective_levels`](crate::Technology::effective_levels) the
//! way a player class does. A class is worth levels, which apply to every ship
//! a side owns; a lifeform bonus applies to one ship type and leaves the rest
//! of the fleet alone. So it rides on [`PartyData`](crate::PartyData) beside
//! the technology levels instead, and reaches the stat calculation as a
//! separate term of the same sum.
//!
//! # Two layers
//!
//! [`LifeformBonuses`] is what combat consumes: resolved percentages per entity
//! type, which is what a caller who already knows its numbers can hand over
//! directly, and what every other current simulator accepts.
//!
//! [`LifeformTechTable`] is the layer above it — which research buffs which
//! ships, and by how much per level. [`BuiltinLifeformTechs`] is the offline
//! implementation bundled with this crate. It is deliberately a trait:
//! Gameforge publishes the whole configuration per universe in
//! `serverData.xml`'s `<lifeformSettings>` block, so integration crates can add
//! a live source without rewriting combat's seam.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::EntityType;

/// Identifier of a lifeform research, matching the ids `OGame` itself uses
/// (`11209`, `12216`, …) so a `serverData.xml` loader needs no translation.
pub type LifeformTechId = u16;

/// What one side's lifeform research adds to a **single** entity type, as
/// percentages of the base stat: `50.0` means `+50%`, the figure in the worked
/// Destroyer example.
///
/// Percentages rather than fractions because every other percentage in a
/// request — `debris_percentage`, `plunder_percentage` — is written that way,
/// and one conversion at the stat calculation is easier to keep straight than
/// two conventions in the same JSON body.
///
/// `cargo` and `speed` are carried and **read by no combat code**. Every
/// lifeform ship research buffs all five stats at the same rate, so leaving
/// them out would mean a later flight or loot model, or the `serverData.xml`
/// loader, having to change this type rather than only its readers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub struct LifeformBonus {
    #[serde(default)]
    pub weapon: f32,
    #[serde(default)]
    pub shield: f32,
    #[serde(default)]
    pub armour: f32,
    #[serde(default)]
    pub cargo: f32,
    #[serde(default)]
    pub speed: f32,
}

impl LifeformBonus {
    /// The same percentage on all five stats, which is what an actual lifeform
    /// research grants — armour, shield, weapon, cargo and speed all move at
    /// one rate.
    #[must_use]
    pub fn uniform(percent: f32) -> Self {
        Self {
            weapon: percent,
            shield: percent,
            armour: percent,
            cargo: percent,
            speed: percent,
        }
    }

    /// Two bonuses on the same ship add. A player's lifeform researches are
    /// empire-wide while the buildings that boost them are per-planet, so the
    /// figure a caller arrives at is already a sum; ships buffed by two species'
    /// trees sum the same way.
    #[must_use]
    fn plus(self, other: Self) -> Self {
        Self {
            weapon: self.weapon + other.weapon,
            shield: self.shield + other.shield,
            armour: self.armour + other.armour,
            cargo: self.cargo + other.cargo,
            speed: self.speed + other.speed,
        }
    }
}

/// One side's lifeform bonuses, keyed by entity type. An entity with no entry
/// has no bonus, which is the common case: no lifeform research in the game
/// touches the Small Cargo, Colony Ship, Espionage Probe, Solar Satellite,
/// Crawler, Pathfinder, Reaper or Deathstar.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(transparent)]
pub struct LifeformBonuses(HashMap<EntityType, LifeformBonus>);

impl LifeformBonuses {
    /// The bonus for one entity type — zero when the side has none, so callers
    /// never branch.
    #[must_use]
    pub fn get(&self, entity_type: EntityType) -> LifeformBonus {
        self.0.get(&entity_type).copied().unwrap_or_default()
    }

    /// True when this side has no lifeform research at all, which is what makes
    /// a request that names none resolve exactly as it did before lifeforms
    /// were modelled.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add a bonus to what an entity type already carries.
    pub fn add(&mut self, entity_type: EntityType, bonus: LifeformBonus) {
        let entry = self.0.entry(entity_type).or_default();
        *entry = entry.plus(bonus);
    }
}

impl FromIterator<(EntityType, LifeformBonus)> for LifeformBonuses {
    fn from_iter<I: IntoIterator<Item = (EntityType, LifeformBonus)>>(iter: I) -> Self {
        let mut bonuses = Self::default();
        for (entity_type, bonus) in iter {
            bonuses.add(entity_type, bonus);
        }
        bonuses
    }
}

/// One lifeform research: which entity types it buffs and by how much per
/// level. Owned rather than borrowed so an implementation that reads
/// `serverData.xml` at runtime can produce these too.
///
/// Which species owns the research is not modelled. It changes nothing about
/// the bonus, `serverData.xml` keys the configuration by id rather than by
/// species, and a field nothing reads is a field that goes quietly wrong. The
/// species is named in a comment beside each row of the built-in table instead.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeformTech {
    pub id: LifeformTechId,
    pub targets: Vec<EntityType>,
    /// Percentage added to each affected stat per researched level. Linear, and
    /// uncapped in game — the brake is cost, not a ceiling.
    pub per_level_percent: f32,
}

/// Where the lifeform configuration comes from.
///
/// [`BuiltinLifeformTechs`] is the offline implementation bundled with this
/// crate. The trait exists because Gameforge publishes the same configuration
/// per universe in `serverData.xml`'s `<lifeformSettings>` block — effect type,
/// target ids, per-level value, growth factor and caps — so a loader can be
/// added by an integration crate without touching combat or its callers.
pub trait LifeformTechTable {
    /// The effect of one research, or `None` when this table does not define
    /// that id.
    fn tech(&self, id: LifeformTechId) -> Option<LifeformTech>;

    /// Every id this table defines.
    fn ids(&self) -> Vec<LifeformTechId>;

    /// The bonuses a set of researched levels is worth.
    ///
    /// `boost_percent` is one number multiplying the raw per-level figures, and
    /// deliberately only a number: in the game it is species experience (`0.1%`
    /// per level, capped at level 100) plus whichever boost buildings a planet
    /// has (Metropolis, Chip Mass Production, High-Performance Transformer,
    /// capped together at `+100%`), and both of those are per planet while the
    /// research is empire-wide. Modelling that is an entire expansion's worth of
    /// data and belongs above this seam, so **the caps are the caller's to
    /// apply** — a figure summed over planets has no single cap to check it
    /// against, and clamping one here would silently understate a real empire.
    /// Pass `0.0` for the raw researched value.
    ///
    /// Ids this table does not define are skipped rather than rejected: a
    /// universe may define research this table has never heard of, and dropping
    /// the battle for it would be worse than fighting it without that bonus.
    fn resolve(&self, levels: &HashMap<LifeformTechId, u8>, boost_percent: f32) -> LifeformBonuses {
        let boost = 1.0 + boost_percent / 100.0;
        let mut bonuses = LifeformBonuses::default();

        for (&id, &level) in levels {
            let Some(tech) = self.tech(id) else { continue };
            let percent = tech.per_level_percent * f32::from(level) * boost;
            for target in tech.targets {
                // Uniform because a lifeform research moves all five stats at
                // one rate. On a defence structure the cargo and speed halves
                // of that are a percentage of zero, which is why the game does
                // not bother to describe them either.
                bonuses.add(target, LifeformBonus::uniform(percent));
            }
        }

        bonuses
    }
}

/// The eight defence structures, 401-408. Missiles (502, 503) are excluded:
/// they are not shot at in a combat round and no lifeform research names them.
const DEFENCES: [EntityType; 8] = [401, 402, 403, 404, 405, 406, 407, 408];

/// Every lifeform research that changes a combat stat, and nothing else.
///
/// Read as `(id, targets, percent per level)`, grouped by the species that owns
/// each research. The rate is `0.3%` per level except where noted, and each
/// research moves armour, shield, weapon, cargo and speed at that one rate.
///
/// Absent on purpose: `12217` Rune Shields, `13217` Experimental Weapons
/// Technology and `14217` Psionic Shield Matrix read like combat research and
/// grant none — they reduce research cost and time. Wiring them to stats is the
/// obvious mistake this comment exists to stop.
const BUILTIN_TECHS: &[(LifeformTechId, &[EntityType], f32)] = &[
    // Humans
    (11209, &[204], 0.3), // Light Fighter Mk II
    (11210, &[206], 0.3), // Cruiser
    (11214, &[211], 0.3), // Bomber
    (11215, &[213], 0.3), // Destroyer
    (11216, &[215], 0.3), // Battlecruiser
    // Rock'tal
    (12208, &[205], 0.3), // Ion Crystal Enhancement, Heavy Fighter
    // Obsidian Shield Reinforcement — the only research in the game that
    // touches defence, and it touches all eight at 0.5% per level.
    (12216, &DEFENCES, 0.5),
    // Mecha — General Overhaul, one research per ship
    (13205, &[204], 0.3), // Light Fighter
    (13208, &[209], 1.0), // Recycler, at 1.0% per level
    (13209, &[206], 0.3), // Cruiser
    (13212, &[207], 0.3), // Battleship
    (13214, &[215], 0.3), // Battlecruiser
    (13215, &[211], 0.3), // Bomber
    (13216, &[213], 0.3), // Destroyer
    // Kaelesh — Overclocking
    (14209, &[205], 0.3), // Heavy Fighter
    (14214, &[203], 1.0), // Large Cargo, at 1.0% per level
    (14216, &[207], 0.3), // Battleship
];

/// The lifeform configuration as it stands in the live game, hardcoded the way
/// the ship stat table is. Self-contained, needs no network at simulation time,
/// and goes stale whenever Gameforge rebalances — which is what
/// [`LifeformTechTable`] exists to make survivable.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinLifeformTechs;

impl LifeformTechTable for BuiltinLifeformTechs {
    fn tech(&self, id: LifeformTechId) -> Option<LifeformTech> {
        BUILTIN_TECHS
            .iter()
            .find(|entry| entry.0 == id)
            .map(|&(id, targets, per_level_percent)| LifeformTech {
                id,
                targets: targets.to_vec(),
                per_level_percent,
            })
    }

    fn ids(&self) -> Vec<LifeformTechId> {
        BUILTIN_TECHS.iter().map(|entry| entry.0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn levels(pairs: &[(LifeformTechId, u8)]) -> HashMap<LifeformTechId, u8> {
        pairs.iter().copied().collect()
    }

    /// The rate every ship research moves at, and the one thing a reader is
    /// most likely to get wrong by reaching for the wiki.
    #[test]
    fn a_ship_research_is_worth_three_tenths_of_a_percent_per_level() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(11215, 20)]), 0.0);

        // Destroyer, 20 levels of the Human research: 20 * 0.3% = 6%.
        assert_relative_eq!(bonuses.get(213).weapon, 6.0, epsilon = 1e-4);
    }

    /// Research is per ship type, so a fleet's other ships must not move.
    #[test]
    fn a_research_buffs_only_the_ships_it_names() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(11215, 20)]), 0.0);

        assert_relative_eq!(bonuses.get(206).weapon, 0.0);
        assert_eq!(bonuses.get(206), LifeformBonus::default());
    }

    /// All five stats at one rate — the reason `cargo` and `speed` are carried
    /// even though no combat code reads them.
    #[test]
    fn a_research_moves_all_five_stats_together() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(11215, 10)]), 0.0);

        assert_eq!(bonuses.get(213), LifeformBonus::uniform(3.0));
    }

    /// Two species buff the Light Fighter, and a player with planets of both
    /// gets both. Summed, because that is how per-planet bonuses combine.
    #[test]
    fn two_researches_on_the_same_ship_add() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(11209, 10), (13205, 10)]), 0.0);

        assert_relative_eq!(bonuses.get(204).weapon, 6.0, epsilon = 1e-4);
    }

    /// Recycler and Large Cargo are the two exceptions to `0.3%`, and both are
    /// the sort of number that gets flattened by a careless edit.
    #[test]
    fn the_recycler_and_large_cargo_researches_are_worth_a_full_percent() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(13208, 10), (14214, 10)]), 0.0);

        assert_relative_eq!(bonuses.get(209).weapon, 10.0, epsilon = 1e-4);
        assert_relative_eq!(bonuses.get(203).cargo, 10.0, epsilon = 1e-4);
    }

    /// Obsidian Shield Reinforcement is the whole of lifeform defence: one
    /// research, all eight structures, half a percent a level.
    #[test]
    fn the_one_defence_research_covers_all_eight_structures() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(12216, 10)]), 0.0);

        for defence in DEFENCES {
            assert_relative_eq!(bonuses.get(defence).shield, 5.0, epsilon = 1e-4);
        }
        // Missiles are not defences for this purpose.
        assert_eq!(bonuses.get(502), LifeformBonus::default());
    }

    /// Species experience and the boost buildings multiply the researched
    /// figure rather than adding to it: 6% at +10% experience is 6.6%, not 16%.
    #[test]
    fn the_boost_multiplies_what_the_levels_are_worth() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(11215, 20)]), 10.0);

        assert_relative_eq!(bonuses.get(213).weapon, 6.6, epsilon = 1e-4);
    }

    /// The three cost-and-time researches are the trap this table is written to
    /// avoid: they sound like combat research and grant zero combat power.
    #[test]
    fn the_cost_reduction_researches_grant_nothing() {
        for id in [12217, 13217, 14217] {
            assert!(BuiltinLifeformTechs.tech(id).is_none(), "{id}");
        }

        let bonuses =
            BuiltinLifeformTechs.resolve(&levels(&[(12217, 50), (13217, 50), (14217, 50)]), 0.0);
        assert!(bonuses.is_empty());
    }

    /// No lifeform research touches these, and claiming otherwise would make
    /// every Deathstar battle wrong.
    #[test]
    fn the_ships_with_no_research_stay_at_their_base_stats() {
        let all: HashMap<_, _> = BuiltinLifeformTechs
            .ids()
            .into_iter()
            .map(|id| (id, 50))
            .collect();
        let bonuses = BuiltinLifeformTechs.resolve(&all, 0.0);

        // Small Cargo, Colony Ship, Espionage Probe, Solar Satellite, Deathstar,
        // Crawler, Reaper, Pathfinder.
        for untouched in [202, 208, 210, 212, 214, 217, 218, 219] {
            assert_eq!(
                bonuses.get(untouched),
                LifeformBonus::default(),
                "{untouched}"
            );
        }
    }

    /// A universe may define research this table has never heard of. Fighting
    /// the battle without that bonus beats refusing to fight it.
    #[test]
    fn an_unknown_research_is_skipped_rather_than_failing() {
        let bonuses = BuiltinLifeformTechs.resolve(&levels(&[(19999, 10), (11215, 10)]), 0.0);

        assert_relative_eq!(bonuses.get(213).weapon, 3.0, epsilon = 1e-4);
    }

    /// No research is listed twice, which a hand-written table invites.
    #[test]
    fn every_id_in_the_builtin_table_is_distinct() {
        let mut ids = BuiltinLifeformTechs.ids();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), count);
    }

    /// Every target has to be a real entity, or the bonus lands on nothing and
    /// nobody notices.
    #[test]
    fn every_target_is_a_known_entity() {
        let entity_db = crate::entities::entity_stats();

        for id in BuiltinLifeformTechs.ids() {
            let tech = BuiltinLifeformTechs.tech(id).expect("listed id resolves");
            for target in tech.targets {
                assert!(entity_db.contains_key(&target), "{id} targets {target}");
            }
        }
    }
}
