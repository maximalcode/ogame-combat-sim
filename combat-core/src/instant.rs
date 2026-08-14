//! v13's instant calculation: deciding a battle without fighting it.
//!
//! `OGame` 13.0.0 replaced the old "a fleet of nothing but espionage probes loses
//! automatically" special case with a general one:
//!
//! > Espionage probes do not automatically lose a battle if they are the only
//! > ship type from one player anymore. Instead, battles will be instantly
//! > calculated only if one side has more than 10.000 times the combined attack
//! > power of the opposing side.
//!
//! Both halves land here. The engine never had the probe rule to remove — 210
//! is a row in the stat table and a rapid fire target and nothing else — and
//! `a_probe_only_fleet_is_not_an_automatic_loss` in
//! `combat-core/tests/instant_calculation.rs` is what keeps it that way. What
//! is new is the short-circuit, and its whole design follows from one sentence:
//! **it may only ever be an optimisation.** A battle it resolves has to come
//! out where the six rounds would have put it — same winner, same losses — or
//! the simulator has two sets of rules and the faster one decides by accident.
//!
//! # Combined attack power
//!
//! One definition, used for both sides: the effective weapon damage of every
//! unit the side brings, summed over the units. *Effective* is the load-bearing
//! word. The figure is read off the [`Entity`](crate::entity::Entity) values
//! the round loop itself shoots with, which is downstream of everything that
//! modifies a stat — technology levels, the player and alliance class bonuses
//! already folded into them by `Technology::effective_levels`, and lifeform
//! percentages applied in `ModifiedStats::calculate`. Summing per unit rather
//! than per type is the same arithmetic as multiplying a type's damage by its
//! count, and it cannot drift from the battle it is deciding.
//!
//! **Defences count towards a defender's attack power.** They shoot; a Plasma
//! Turret's 3000 is damage the attacker has to survive exactly as a
//! Battleship's 1000 is. The changelog says "one side" and "the opposing side",
//! not "fleet", and below [`Party`] the engine draws no line between a ship and
//! a defence anyway. The case the ratio gets asked about most — a real fleet
//! arriving over a planet — is a case where the defender's whole attack power
//! *is* defences, and reading it as zero would fire the short-circuit on
//! battles a rocket launcher can still take ships out of.
//!
//! # Why the ratio is necessary but not sufficient
//!
//! The changelog's condition alone contradicts this engine's own rounds, and
//! not in a corner nobody visits. Three battles say why:
//!
//! - **250 Light Fighters against one Large Shield Dome.** That is
//!   `combat-core/tests/common/mod.rs`, the fixture the stat-modifier tests
//!   share, run at Weapons 9: a fighter's shot is 95 there, so 250 of them are
//!   worth 23,750 attack power against the dome's own 1. The ratio is met —
//!   2.4 times over the 10,000 threshold — but 95 is under 1% of the dome's
//!   10,000 shield, so every shot bounces and the correct answer is six rounds
//!   ending in a draw with nobody having lost anything. An instant calculation
//!   on the ratio alone would report the dome destroyed.
//! - **One Solar Satellite against one Espionage Probe.** The probe's attack
//!   power is 0, so the ratio is met by any armed opponent whatsoever. The
//!   satellite's shot is worth 1 and the probe has 100 hull: six rounds leave
//!   the probe alive on 94 hull, which is a draw, not an annihilation.
//! - **600,001 Light Fighters against one Plasma Turret.** 30,000,050 attack
//!   power against 3,000 clears the threshold — barely, which is the point:
//!   600,000 fighters would be exactly ten thousand times over and the
//!   changelog says "more than", so one more is what it takes — and the
//!   fighters do win. But the turret gets one shot of 3,000 in before it dies,
//!   which lands on whichever fighter's 10 shield it is aimed at for thousands
//!   of times the shield's worth, and the fought battle costs the attacker
//!   exactly one fighter (`cargo run -p combat-cli -- sim -a "lf:600001" -d
//!   "plasmaturret:1" --tech 0 -n 1 --rounds` puts it on the board). "The
//!   winner loses nothing" is false here, and a ratio-only rule would have
//!   reported zero.
//!
//! So the ratio is the gate, and three further conditions — each one read
//! straight off the engine's own damage rule rather than invented — decide
//! whether the shortcut is allowed to speak for the rounds:
//!
//! 1. the loser's fire must not register on the winner at all, so the winner
//!    provably loses nothing (the third battle above);
//! 2. the winner's fire must register on everything the loser has, so the
//!    bounce rule cannot save it (the first battle above);
//! 3. the winner's firepower must overwhelm the loser's hitpoints, so the wipe
//!    is not merely likely (the second battle above).
//!
//! When any of them fails the battle takes the ordinary path and is simulated,
//! which is slower and *right*. That makes this engine's short-circuit strictly
//! narrower than the rule as the changelog states it: a battle `OGame` would
//! instant-calculate may be fought here round by round. That is the deliberate
//! direction to be wrong in, because the cost is time and the alternative cost
//! is a wrong answer.
//!
//! # The one number that is not from the changelog
//!
//! Condition 3 needs a margin: "the winner's firepower exceeds the loser's
//! hitpoints" is not enough on its own, because targets are picked at random
//! and shots land on units that are already dead. Rather than invent a second
//! constant, it reuses the one the changelog supplies —
//! [`WIPE_CERTAINTY_MARGIN`] is [`INSTANT_CALCULATION_RATIO`] — and the reuse
//! is conservative by construction: the condition is `attack_power >= MARGIN *
//! loser.hitpoints`, so a *larger* margin is a *harder* bar to clear, and too
//! large a margin means the battle gets simulated more often than it strictly
//! needed to — the correct answer, arrived at the slow way. The dangerous
//! direction is the other one: too small a margin would call a wipe-out
//! certain when six rounds of shots wasted on the already-dead would actually
//! have left something standing. Reusing the changelog's 10,000× — a figure
//! this module did not get to pick to be conveniently large — rather than
//! inventing something smaller for the purpose keeps the margin on the safe
//! side of that line.
//!
//! With it, the stat table bounds the waste, at Weapons 0: the frailest thing
//! in the game has 100 hitpoints and the hardest *base* shot in the table is a
//! Deathstar's 200,000, so a winner that brings ten thousand times the loser's
//! hitpoints in base weapon damage is bringing at least five shots per enemy
//! unit per round however it is composed, and six rounds of five shots each
//! leave nothing standing. That specific arithmetic does not survive
//! technology: [`SideProfile::of`] reads *effective* weapon power, and a
//! Deathstar at Weapons 255 fires 5,300,000 rather than 200,000, which loosens
//! the same condition to as little as 5.3 loser units per winner unit — 0.19
//! shots per enemy unit per round, not five. The conclusion is not shown to
//! survive by this counting argument at tech the type system allows; what
//! carries it there instead is that a side clearing the margin at that kind of
//! weapon power is landing hits that are themselves enormously overkill per
//! target, one-shotting whatever they touch rather than needing several
//! rounds of accumulated damage — a different argument, and one this module
//! does not attempt to make precise. What the margin cannot be, at any tech
//! level, is *proved*, and
//! `no_battle_the_rule_decides_disagrees_with_the_battle_it_replaces` is the
//! answer to that: fleets nobody chose, fought both ways.

use crate::combat::Party;

/// The changelog's threshold, and the only number in this module that comes
/// from outside the engine: "more than 10.000 times the combined attack power
/// of the opposing side". More than, so the comparison is strict — a side with
/// exactly ten thousand times the other's fights its battle.
const INSTANT_CALCULATION_RATIO: f64 = 10_000.0;

/// How far the winner's firepower has to exceed the loser's total hitpoints
/// before the wipe counts as certain rather than likely.
///
/// Deliberately the same number as [`INSTANT_CALCULATION_RATIO`] rather than a
/// second constant of its own: nothing sources a margin for this, and the
/// module documentation explains why reusing the sourced one is both honest and
/// conservative.
const WIPE_CERTAINTY_MARGIN: f64 = INSTANT_CALCULATION_RATIO;

/// Which side an instant calculation wipes out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Attackers,
    Defenders,
}

/// Whether a battle is allowed to be decided by the rule.
///
/// Every battle the engine fights answers `Applied`. `Skipped` exists so the
/// same battle can be fought down the round loop and the two results compared —
/// see [`Combat::simulate_single_through_the_rounds`](crate::Combat::simulate_single_through_the_rounds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstantCalculation {
    Applied,
    Skipped,
}

impl InstantCalculation {
    /// Empty out whichever side the rule annihilates, if it annihilates one.
    pub(crate) fn apply(self, attackers: &mut Party, defenders: &mut Party) {
        if self == Self::Skipped {
            return;
        }

        match annihilated_side(attackers, defenders) {
            Some(Side::Attackers) => attackers.annihilate(),
            Some(Side::Defenders) => defenders.annihilate(),
            None => {}
        }
    }
}

/// The side v13's rule annihilates without a shot being fired, if either.
///
/// Both directions are asked, because the rule is about "one side" and a
/// defender with a planet full of Plasma Turrets can be the overwhelming one.
/// They cannot both answer yes: each requires the other's attack power to be
/// ten thousand times smaller.
fn annihilated_side(attackers: &Party, defenders: &Party) -> Option<Side> {
    let attacker_profile = SideProfile::of(attackers);
    let defender_profile = SideProfile::of(defenders);

    if attacker_profile.annihilates(&defender_profile) {
        Some(Side::Defenders)
    } else if defender_profile.annihilates(&attacker_profile) {
        Some(Side::Attackers)
    } else {
        None
    }
}

/// Everything about one side the rule needs, in one pass over its units.
///
/// Extremes rather than averages, and that is the point: a side is only inert
/// if its *best* shot cannot register on the enemy's *weakest* shield, and only
/// lethal if its *worst* armed shot registers on the enemy's *toughest* one.
/// Averages would let one Deathstar hide behind a million Small Cargos.
struct SideProfile {
    /// Combined attack power: every unit's effective weapon damage, summed.
    attack_power: f64,
    /// Shield plus hull over every unit — what the other side has to chew
    /// through to leave nothing standing.
    hitpoints: f64,
    /// The hardest single shot this side can fire.
    strongest_shot: f32,
    /// The softest shot from a unit that is armed at all. Unarmed units —
    /// probes, and anything a lifeform table ever floors to zero — are skipped
    /// rather than counted as a shot of 0: they fire nothing that can bounce,
    /// so letting them set this floor would decline the shortcut on exactly the
    /// battle the changelog is about, a real fleet with its probes attached.
    weakest_armed_shot: Option<f32>,
    /// The smallest maximum shield on the side — the easiest unit to register a
    /// hit on.
    smallest_shield: f32,
    /// The largest maximum shield on the side — the hardest one to register a
    /// hit on.
    largest_shield: f32,
}

impl SideProfile {
    fn of(party: &Party) -> Self {
        let mut profile = Self {
            attack_power: 0.0,
            hitpoints: 0.0,
            strongest_shot: 0.0,
            weakest_armed_shot: None,
            smallest_shield: f32::INFINITY,
            largest_shield: 0.0,
        };

        for entity in &party.entities {
            let shot = entity.weapon_power as f32;

            profile.attack_power += f64::from(shot);
            profile.hitpoints += f64::from(entity.max_shield) + f64::from(entity.max_hull);
            profile.strongest_shot = profile.strongest_shot.max(shot);
            if shot > 0.0 {
                profile.weakest_armed_shot =
                    Some(profile.weakest_armed_shot.map_or(shot, |w| w.min(shot)));
            }
            profile.smallest_shield = profile.smallest_shield.min(entity.max_shield);
            profile.largest_shield = profile.largest_shield.max(entity.max_shield);
        }

        profile
    }

    /// Whether this side wipes `loser` out without the rounds being fought.
    ///
    /// The four conditions in the order they are cheapest to reject on. Each
    /// one is spelled out at its site; the module documentation is where the
    /// battles that forced them are.
    fn annihilates(&self, loser: &SideProfile) -> bool {
        // The changelog's rule. Strict, and written as a multiplication rather
        // than a ratio so that a loser with no attack power at all — a fleet of
        // probes, which is the case the rule was written for — is answered by
        // `self.attack_power > 0.0` instead of by a division by zero. Two sides
        // with no attack power between them fail it (0 is not more than 10,000
        // times 0) and are simulated, which is right: neither can destroy
        // anything, and six rounds of shots that do nothing is the draw with no
        // losses that the rounds already report.
        if self.attack_power <= INSTANT_CALCULATION_RATIO * loser.attack_power {
            return false;
        }

        // The loser must not be able to cost the winner a single unit. Its best
        // shot against the winner's frailest shield has to bounce — and a shot
        // that bounces is absorbed entirely, so no shield is dented, no hull is
        // touched and no explosion is rolled, however many times it is fired
        // and whatever rapid fire multiplies it by.
        if shot_registers(loser.strongest_shot, self.smallest_shield) {
            return false;
        }

        // The winner's fire must reach everything the loser has. One unit type
        // whose shield swallows the winner's softest armed shot is enough to
        // leave survivors, and survivors make this a draw rather than the
        // annihilation the short-circuit is about to report.
        let Some(weakest_armed_shot) = self.weakest_armed_shot else {
            return false;
        };
        if !shot_registers(weakest_armed_shot, loser.largest_shield) {
            return false;
        }

        // And it must be overwhelming rather than merely sufficient, because
        // targets are chosen at random and shots are spent on units that are
        // already dead. See the module documentation for why the margin is the
        // changelog's own number.
        self.attack_power >= WIPE_CERTAINTY_MARGIN * loser.hitpoints
    }
}

/// Whether a shot of `shot` damage does anything at all to a unit whose maximum
/// shield is `shield`.
///
/// This mirrors the bounce rule in `apply_damage_fast`, in the same `f32` and
/// with the same expression, deliberately: a rearrangement into `shot * 100 >=
/// shield` is the same inequality in real arithmetic and not always the same
/// one in floating point, and this predicate is only useful if it agrees with
/// the damage it is predicting. If that rule ever changes, this changes with
/// it.
///
/// The two degenerate cases fall out of the arithmetic and match the engine. A
/// shot into a unit with no shield at all divides by zero, comes out infinite
/// and registers — which is what the damage function does with it, straight
/// into the hull. An unarmed unit shooting an unshielded one is `0.0 / 0.0`,
/// is not greater than one, and registers nothing — which is also what the
/// damage function does with it.
fn shot_registers(shot: f32, shield: f32) -> bool {
    (shot / shield * 100.0).floor() >= 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;
    // Every figure below is a small integer that arrives in an `f32` intact, so
    // the comparisons are exact today. A relative comparison says what the test
    // means without depending on that staying true.
    use approx::assert_relative_eq;
    use std::collections::HashMap;

    /// A side of `count` identical units, described the way the rule reads
    /// them. Building profiles directly rather than parties keeps these tests
    /// on the decision and off the stat table.
    fn side(count: f64, weapon: f32, shield: f32, hull: f32) -> SideProfile {
        SideProfile {
            attack_power: count * f64::from(weapon),
            hitpoints: count * (f64::from(shield) + f64::from(hull)),
            strongest_shot: weapon,
            weakest_armed_shot: (weapon > 0.0).then_some(weapon),
            smallest_shield: shield,
            largest_shield: shield,
        }
    }

    /// A Light Fighter at Weapons 0: the workhorse of these tests, and frail
    /// enough that a defence's shot is never inert against it.
    fn light_fighters(count: f64) -> SideProfile {
        side(count, 50.0, 10.0, 400.0)
    }

    /// Espionage Probes: no weapon, no shield, 100 hull. The fleet the v13 rule
    /// was written about.
    fn probes(count: f64) -> SideProfile {
        side(count, 0.0, 0.0, 100.0)
    }

    #[test]
    fn a_fleet_annihilates_probes_it_overwhelms() {
        // 100,000 fighters: 5,000,000 attack power against 100 hitpoints, which
        // clears the margin fifty times over.
        assert!(light_fighters(100_000.0).annihilates(&probes(1.0)));
    }

    #[test]
    fn probes_do_not_annihilate_anything() {
        assert!(!probes(1_000_000.0).annihilates(&light_fighters(1.0)));
    }

    /// Zero attack power on the losing side is answered without dividing by it,
    /// and zero on both sides is not a ten-thousand-fold mismatch.
    #[test]
    fn two_sides_with_no_attack_power_are_not_a_mismatch() {
        assert!(!probes(1.0).annihilates(&probes(1_000_000.0)));
        assert!(!probes(1_000_000.0).annihilates(&probes(1.0)));
    }

    /// "More than 10.000 times", so exactly ten thousand times is not enough.
    /// A Solar Satellite is worth 1 attack power and has a shield of 1, which a
    /// Destroyer's shot of 2000 flattens without registering the other way.
    #[test]
    fn exactly_ten_thousand_times_is_not_more_than_ten_thousand_times() {
        let satellite = side(1.0, 1.0, 1.0, 200.0);
        let destroyers = |count: f64| side(count, 2000.0, 500.0, 1100.0);

        // 5 Destroyers are worth exactly 10,000 attack power against the
        // satellite's 1. One more Destroyer clears the threshold — and then
        // fails on the margin instead, which is the next condition doing its
        // own job rather than this one being met.
        assert!(!destroyers(5.0).annihilates(&satellite));
        assert!(!destroyers(6.0).annihilates(&satellite));
        assert!(destroyers(2_000_000.0).annihilates(&satellite));
    }

    /// The bounce rule, which is the condition the shared fixture's battle
    /// forced: a side that clears the ratio against a Large Shield Dome may
    /// still be unable to scratch it.
    ///
    /// The attackers here carry a Battleship's 200 shield rather than a Light
    /// Fighter's 10, so that the dome's own shot of 1 bounces and the answer
    /// comes from the condition under test instead of from the one before it.
    #[test]
    fn shots_that_bounce_off_the_loser_decline_the_shortcut() {
        let dome = side(1.0, 1.0, 10_000.0, 10_000.0);
        let attackers = |count: f64, weapon: f32| side(count, weapon, 200.0, 6000.0);

        // 95 is under 1% of the dome's 10,000 shield, so every shot bounces.
        assert!(!attackers(250.0, 95.0).annihilates(&dome));
        // 100 registers — and 25,000 attack power is still nowhere near ten
        // thousand times the dome's 20,000 hitpoints, so this one is simulated
        // for the next reason instead.
        assert!(!attackers(250.0, 100.0).annihilates(&dome));
        // Enough of them and it is a foregone conclusion.
        assert!(attackers(10_000_000.0, 100.0).annihilates(&dome));
    }

    /// A loser whose shots still register keeps its ability to take units with
    /// it, so the rule declines however lopsided the ratio is. Fifty Rocket
    /// Launchers against a million Light Fighters: 4000 attack power against
    /// 50,000,000, and the fighters would still bury fifty of their own.
    #[test]
    fn a_loser_that_can_still_shoot_back_declines_the_shortcut() {
        let launchers = side(50.0, 80.0, 20.0, 200.0);
        assert!(!light_fighters(1_000_000.0).annihilates(&launchers));
    }

    /// Firepower that clears the ratio but not the loser's hull: six rounds of
    /// a Solar Satellite's single point of damage leave an Espionage Probe
    /// alive on 94 hull, and a draw is not an annihilation.
    #[test]
    fn firepower_that_cannot_finish_the_job_declines_the_shortcut() {
        assert!(!side(1.0, 1.0, 1.0, 200.0).annihilates(&probes(1.0)));
    }

    /// The profile is read off the units the round loop shoots with, and an
    /// unarmed unit among them must not be read as a shot of zero that bounces
    /// off everything: a fleet with its probes attached is the ordinary case,
    /// not an edge one.
    #[test]
    fn a_profile_reads_the_units_the_battle_will_use() {
        let party = Party {
            entities: (0..10)
                .map(|_| Entity::new(207, 1000, 200.0, 6000.0))
                .chain(std::iter::once(Entity::new(210, 0, 0.0, 100.0)))
                .collect(),
            rapid_fire_map: HashMap::new(),
        };

        let profile = SideProfile::of(&party);

        // Ten Battleships' worth of firepower; the probe adds nothing to it.
        assert_relative_eq!(profile.attack_power, 10_000.0);
        // ...but its hull is still something the other side has to destroy.
        assert_relative_eq!(profile.hitpoints, 62_100.0);
        // The probe sets the frailest shield, because it is the easiest thing
        // on the side to land a hit on...
        assert_relative_eq!(profile.smallest_shield, 0.0);
        // ...and does not set the softest shot, because it fires nothing.
        assert_relative_eq!(profile.weakest_armed_shot.unwrap(), 1000.0);
        assert_relative_eq!(profile.strongest_shot, 1000.0);
        assert_relative_eq!(profile.largest_shield, 200.0);
    }

    #[test]
    fn the_bounce_predicate_matches_the_damage_rule() {
        // 1% of the shield exactly, which the damage rule counts as landing.
        assert!(shot_registers(100.0, 10_000.0));
        // A hair under, which it does not.
        assert!(!shot_registers(99.0, 10_000.0));
        // No shield to bounce off.
        assert!(shot_registers(1.0, 0.0));
        // Nothing fired at nothing.
        assert!(!shot_registers(0.0, 0.0));
    }
}
