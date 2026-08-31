//! The fixture the stat-modifier tests share: [`FIGHTERS`] Light Fighters
//! attacking a single Large Shield Dome.
//!
//! `class_bonuses.rs` and `lifeform_bonuses.rs` ask the same question of two
//! different mechanisms — did this modifier reach the battle, and by how much —
//! so they resolve the same battle and read the same summary out of it. Each
//! keeps its own `resolve`, because they build different requests and the point
//! of each is the field it sets. What lives here is the part that would drift if
//! it were written twice: the battle, and why it answers the question.
//!
//! The dome's weapon is 1, which is under 1% of a Light Fighter's shield, so its
//! shots bounce and the attacker cannot lose a ship however long the battle
//! lasts. Nothing else in the battle rolls a die that matters: there is one
//! target, so target selection is fixed, neither ship has rapid fire against the
//! other, and the only unit that ever takes hull damage dies within the round it
//! starts taking them. Every simulation therefore agrees, and the result turns
//! on one comparison — whether a fighter's shot clears 1% of the dome's shield:
//!
//! - below it, the shot bounces off entirely, the dome is untouchable, and six
//!   rounds end in a draw with nobody having lost anything;
//! - at or above it, the shield comes down inside the first round and 250
//!   fighters flatten the dome before it can do anything about it.
//!
//! The dome's shield is 10,000 at Shielding 0, putting the line at 100 — which
//! is a Light Fighter at Weapons 10, and one effective level either side of it
//! changes the answer, whichever side of the battle moved it.

use combat_types::{CombatResults, EntityType, FleetComposition};

pub const LIGHT_FIGHTER: EntityType = 204;
pub const LARGE_SHIELD_DOME: EntityType = 408;

/// Enough fighters to bring a dome down inside one round once their shots
/// register at all — the shield absorbs the first hundred of them.
pub const FIGHTERS: u32 = 250;

/// The highest Weapons level whose shots still bounce: a Light Fighter's 50
/// becomes 95, and 95 is under 1% of the dome's 10,000 shield.
pub const WEAPONS_THAT_BOUNCE: u8 = 9;

/// One level up, the shot is worth exactly 100 and registers.
pub const WEAPONS_THAT_LAND: u8 = 10;

/// The fixture repeats, and a handful of runs of a battle with no randomness in
/// it is enough — if anything here were random these tests would flap loudly
/// rather than quietly.
pub const SIMULATIONS: u32 = 5;

/// Everything about the shared battle a stat modifier could possibly move.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub attacker_wins: u32,
    pub defender_wins: u32,
    pub draws: u32,
    pub rounds: u8,
    pub attacker_losses: FleetComposition,
    pub defender_losses: FleetComposition,
}

/// Everything a run of the fixture is allowed to say, read out of the first
/// simulation because every simulation of it agrees.
pub fn summarise(results: &CombatResults) -> Outcome {
    let first = &results.results[0];
    Outcome {
        attacker_wins: results.attacker_wins,
        defender_wins: results.defender_wins,
        draws: results.draws,
        rounds: first.rounds,
        attacker_losses: first.attacker_losses.clone(),
        defender_losses: first.defender_losses.clone(),
    }
}
