//! Separate diagnostic input: observed effective stats, never inferred modifiers.
use crate::{Combat, ModifiedStats, SingleCombatResult, StatsCache};
use combat_types::{PartyData, entities::entity_stats};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedUnit {
    pub count: u32,
    pub weapon: u32,
    pub shield: f32,
    /// Effective hull points, not structural integrity. Convert structural
    /// integrity to hull explicitly at the source boundary, dividing by ten.
    pub hull: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedSlot {
    pub units: BTreeMap<u16, ReportedUnit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedBattle {
    pub attackers: Vec<ReportedSlot>,
    pub defenders: Vec<ReportedSlot>,
    pub rapid_fire: bool,
}

impl ReportedBattle {
    /// Bound exact simulations rather than silently downscaling observations.
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut total = 0u64;
        for side in [&self.attackers, &self.defenders] {
            if side.is_empty() || side.len() > 255 {
                return Err("reported battle requires 1..255 slots per side");
            }
            for slot in side {
                if slot.units.is_empty() {
                    return Err("reported slot must contain units");
                }
                for (id, unit) in &slot.units {
                    if !entity_stats().contains_key(id) {
                        return Err("reported unit type is unsupported");
                    }
                    if unit.count == 0
                        || !unit.shield.is_finite()
                        || unit.shield < 0.0
                        || !unit.hull.is_finite()
                        || unit.hull <= 0.0
                    {
                        return Err(
                            "reported units require positive counts/hull and finite nonnegative shields",
                        );
                    }
                    total += u64::from(unit.count);
                }
            }
        }
        if total > 5_000_000 {
            return Err(
                "reported battle exceeds the exact-comparison limit of 5 million units; no downscaling was applied",
            );
        }
        Ok(())
    }
}

impl Combat {
    /// Use supplied effective statistics once, with the ordinary slot combat
    /// loop and rapid-fire table. No technology or lifeform values are inferred.
    pub fn simulate_reported(
        &self,
        battle: &ReportedBattle,
    ) -> Result<SingleCombatResult, &'static str> {
        battle.validate()?;
        let prepare = |side: &[ReportedSlot]| {
            side.iter()
                .map(|slot| {
                    let data = PartyData {
                        entities: slot.units.iter().map(|(&id, u)| (id, u.count)).collect(),
                        ..PartyData::default()
                    };
                    let stats = StatsCache::reported(
                        slot.units
                            .iter()
                            .map(|(&id, u)| {
                                (
                                    id,
                                    ModifiedStats {
                                        weapon: u.weapon,
                                        shield: u.shield,
                                        hull: u.hull,
                                    },
                                )
                            })
                            .collect(),
                    );
                    (data, stats)
                })
                .collect::<Vec<_>>()
        };
        Ok(self.resolve_slots(
            &prepare(&battle.attackers),
            &prepare(&battle.defenders),
            battle.rapid_fire,
            false,
            &mut rand::rngs::SmallRng::from_os_rng(),
        ))
    }
}
