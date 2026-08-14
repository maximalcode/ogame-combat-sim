//! Human-readable output.
//!
//! Everything here builds a `String` rather than printing, so the shape of the
//! output is testable without capturing stdout.
//!
//! One deliberate omission: there is no machine-readable mode. `--json` is a
//! separate piece of work, and half-doing it — printing something JSON-ish that
//! nobody promised to keep stable — is worse than not having it.

use std::fmt::Write as _;
use std::path::Path;

use combat_fixtures::{Evaluation, NumberCheck};
use combat_types::entities::entity_stats;
use combat_types::names::{ENTITY_INFO, name_of};
use combat_types::{
    BattleType, CombatReport, CombatRequest, CombatResults, EconomicSummary, EntityType,
    FleetComposition, ResourceCost, SimulationResult, Technology,
};

/// Group digits so seven-figure resource totals can be read at a glance.
fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Signed variant for profits, which are genuinely negative about half the time.
fn signed_thousands(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "+" };
    format!("{sign}{}", thousands(value.unsigned_abs()))
}

/// Entity ids in a fixed order so two runs of the same battle print the same
/// way. `FleetComposition` is a `HashMap`, whose iteration order is not.
fn sorted_ids(fleet: &FleetComposition) -> Vec<EntityType> {
    let mut ids: Vec<EntityType> = fleet.keys().copied().collect();
    ids.sort_unstable();
    ids
}

fn display_name(entity_type: EntityType) -> String {
    name_of(entity_type).map_or_else(|| format!("Entity {entity_type}"), ToOwned::to_owned)
}

// Column layouts for the two tables. Macros rather than consts because a
// format string has to be a literal at the call site — a `&str` const would be
// printed, not applied. Stated once each so a header and its rows cannot drift.
macro_rules! round_row {
    ($out:expr, $($cell:expr),+ $(,)?) => {
        let _ = writeln!($out, "  {:>5}  {:>21}  {:>21}  {:>14}  {:>14}", $($cell),+);
    };
}

macro_rules! entity_row {
    ($out:expr, $($cell:expr),+ $(,)?) => {
        let _ = writeln!(
            $out,
            "{:>4}  {:<24}{:<22}{:>9}{:>8}{:>11}{:>11}{:>11}{:>11}{:>11}",
            $($cell),+
        );
    };
}

/// One label-and-value line. Every aligned line in the report goes through
/// here, so the two column widths are stated once instead of at each call site.
fn row(out: &mut String, indent: &str, label: &str, value: &str) {
    let _ = writeln!(out, "{indent}{label:<24}{value:>12}");
}

fn write_fleet(out: &mut String, fleet: &FleetComposition, indent: &str) {
    if fleet.values().all(|&count| count == 0) {
        let _ = writeln!(out, "{indent}(none)");
        return;
    }
    for id in sorted_ids(fleet) {
        let count = fleet[&id];
        if count == 0 {
            continue;
        }
        row(out, indent, &display_name(id), &thousands(count.into()));
    }
}

fn write_cost(out: &mut String, cost: &ResourceCost, indent: &str) {
    row(out, indent, "Metal", &thousands(cost.metal));
    row(out, indent, "Crystal", &thousands(cost.crystal));
    row(out, indent, "Deuterium", &thousands(cost.deuterium));
    row(out, indent, "Total", &thousands(cost.total()));
}

fn tech_summary(tech: &Technology) -> String {
    format!("{}/{}/{}", tech.weapon, tech.shield, tech.armour)
}

/// Spelled out rather than `{:?}` on the enum: the variant names are
/// `FleetVsFleet`, and a report is not the place to show a user our casing.
fn battle_type_label(battle_type: &BattleType) -> &'static str {
    match battle_type {
        BattleType::FleetVsFleet => "Fleet vs fleet",
        BattleType::FleetVsDefense => "Fleet vs defence",
        BattleType::Mixed => "Mixed fleet and defence",
        BattleType::MissileAttack => "Missile attack",
        BattleType::MoonDestruction => "Moon destruction",
    }
}

/// The report for a run of simulations.
///
/// Everything from `results` is exact; everything from `report` is the average
/// across the run, which is why the losses and economics sections say so. A
/// single simulation's report would be a battle; this is a distribution.
#[must_use]
pub fn render_report(
    request: &CombatRequest,
    results: &CombatResults,
    report: &CombatReport,
) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "{}\n", battle_type_label(&report.battle_type));

    // The levels come from the report rather than the request because the
    // report carries the effective ones: a General fights three levels above
    // his research in a Warrior alliance, and printing what he researched
    // would not describe the battle underneath it.
    let _ = writeln!(
        out,
        "Attacker — weapon/shield/armour {}",
        tech_summary(&report.attacker.technology)
    );
    write_fleet(&mut out, &request.attacker.entities, "  ");
    let _ = writeln!(
        out,
        "\nDefender — weapon/shield/armour {}",
        tech_summary(&report.defender.technology)
    );
    write_fleet(&mut out, &request.defender.entities, "  ");

    write_outcome(&mut out, results);
    write_fleet_changes(&mut out, report);
    write_economics(&mut out, &report.economics);

    out
}

/// Who won, how often, and how long it took.
fn write_outcome(out: &mut String, results: &CombatResults) {
    let _ = writeln!(
        out,
        "\n{} simulation{} in {} ms\n",
        thousands(results.simulations.into()),
        if results.simulations == 1 { "" } else { "s" },
        thousands(results.duration_ms)
    );

    let mut rate = |label: &str, rate: f64, count: u32| {
        let _ = writeln!(
            out,
            "  {label:<24}{:>11.1}%  ({})",
            rate * 100.0,
            thousands(count.into())
        );
    };
    rate(
        "Attacker wins",
        results.attacker_win_rate(),
        results.attacker_wins,
    );
    rate(
        "Defender wins",
        results.defender_win_rate(),
        results.defender_wins,
    );
    rate("Draws", results.draw_rate(), results.draws);

    let _ = writeln!(
        out,
        "  {:<24}{:>12.2}",
        "Average rounds", results.average_rounds
    );
}

/// What each side lost and what walked away, averaged over the run.
fn write_fleet_changes(out: &mut String, report: &CombatReport) {
    let _ = writeln!(out, "\nAverage losses — attacker");
    write_fleet(out, &report.attacker_losses.ships, "  ");
    let _ = writeln!(out, "\nAverage losses — defender");
    write_fleet(out, &report.defender_losses.ships, "  ");

    let _ = writeln!(out, "\nAverage survivors — attacker");
    write_fleet(out, &report.attacker_fleet_end.ships, "  ");
    let _ = writeln!(out, "\nAverage survivors — defender");
    write_fleet(out, &report.defender_fleet_end.ships, "  ");
}

/// Debris, plunder and the bottom line.
///
/// The plunder section is skipped when there is none, which is the common case:
/// loot needs `--planet`, and a battle between two fleets in space has no planet
/// to take anything from. Deuterium debris is skipped on the same grounds — it
/// is a per-universe option most universes leave off, so a permanent `0` row
/// would be noise everywhere it is not enabled.
fn write_economics(out: &mut String, economics: &EconomicSummary) {
    let _ = writeln!(out, "\nDebris field (average)");
    row(out, "  ", "Metal", &thousands(economics.debris_field.metal));
    row(
        out,
        "  ",
        "Crystal",
        &thousands(economics.debris_field.crystal),
    );
    if economics.debris_field.deuterium > 0 {
        row(
            out,
            "  ",
            "Deuterium",
            &thousands(economics.debris_field.deuterium),
        );
    }
    let _ = writeln!(
        out,
        "  {:<24}{:>11.1}%",
        "Moon chance", economics.moon_chance
    );
    if let Some(harvest) = &economics.harvest_info {
        row(
            out,
            "  ",
            "Recyclers to harvest",
            &thousands(harvest.recyclers_needed.into()),
        );
    }

    let plunder = &economics.plunder;
    if plunder.total() > 0 {
        let _ = writeln!(out, "\nPlunder (average)");
        write_cost(
            out,
            &ResourceCost {
                metal: plunder.metal,
                crystal: plunder.crystal,
                deuterium: plunder.deuterium,
            },
            "  ",
        );
    }

    let _ = writeln!(out, "\nAverage profit");
    row(
        out,
        "  ",
        "Attacker",
        &signed_thousands(economics.attacker_profit),
    );
    row(
        out,
        "  ",
        "Defender",
        &signed_thousands(economics.defender_profit),
    );
}

/// The round-by-round breakdown of one battle.
///
/// `build_summary_report` sets `round_details` to `None` — an average has no
/// meaningful per-round narrative — so this reads a single [`SimulationResult`]
/// instead, and the header says which one. Presenting one battle's rounds as if
/// they were the run's would be the actual mistake here.
///
/// Always the run's first battle. Any single battle is as representative as any
/// other, and a parameter for choosing one would be a knob nothing turns.
#[must_use]
pub fn render_rounds(result: &SimulationResult, total: u32) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        "\nRounds — simulation 1 of {} (one battle, not an average)\n",
        thousands(total.into())
    );

    let Some(rounds) = result.round_details.as_deref() else {
        let _ = writeln!(out, "  (no round detail was recorded for this battle)");
        return out;
    };

    round_row!(
        out,
        "Round",
        "Attackers",
        "Defenders",
        "Att. damage",
        "Def. damage"
    );

    for round in rounds {
        round_row!(
            out,
            round.round_number,
            survivors(round.attackers_start, round.attackers_end),
            survivors(round.defenders_start, round.defenders_end),
            round
                .attacker_damage
                .map_or_else(|| "-".to_owned(), thousands),
            round
                .defender_damage
                .map_or_else(|| "-".to_owned(), thousands),
        );
    }

    out
}

fn survivors(start: u32, end: u32) -> String {
    format!(
        "{} -> {} (-{})",
        thousands(start.into()),
        thousands(end.into()),
        thousands(u64::from(start.saturating_sub(end)))
    )
}

/// The entity table: what `-a` and `-d` will accept, and what each unit is worth.
///
/// Five of `EntityStats`' twelve fields are left out, and the footer says so:
/// the two rapid-fire tables are maps that would need a page rather than a
/// column, and base speed and fuel consumption belong to flight, not battle.
#[must_use]
pub fn render_entities() -> String {
    let stats = entity_stats();
    let mut out = String::new();

    entity_row!(
        out, "ID", "Name", "Aliases", "Weapon", "Shield", "Armour", "Metal", "Crystal", "Deut",
        "Cargo"
    );

    for entity in ENTITY_INFO {
        // `every_named_entity_exists_in_the_stats_table` in combat-types makes
        // this lookup total; the fallback keeps a future gap from panicking a
        // read-only command.
        let Some(stat) = stats.get(&entity.entity_type) else {
            continue;
        };
        entity_row!(
            out,
            entity.entity_type,
            entity.name,
            entity.aliases.join(", "),
            thousands(stat.weapon.into()),
            thousands(stat.shield.into()),
            thousands(stat.armour.into()),
            thousands(stat.cost_metal.into()),
            thousands(stat.cost_crystal.into()),
            thousands(stat.cost_deuterium.into()),
            thousands(stat.cargo_capacity.into()),
        );
    }

    let _ = writeln!(
        out,
        "\nNames are matched without case or punctuation, so \"Light Fighter\", \
         \"light-fighter\" and \"lf\" are the same ship.\nNot shown: rapid-fire tables, \
         base speed and fuel consumption — none of them affect combat."
    );

    out
}

// One layout for the header and its rows, so the two cannot drift. A macro
// rather than a const because a format string must be a literal where it is
// used — the same reason render.rs states its tables this way.
macro_rules! metric_row {
    ($out:expr, $($cell:expr),+ $(,)?) => {
        let _ = writeln!($out, "  {:<26} {:>14} {:>14} {:>12} {:>12}  {:<21} {}", $($cell),+);
    };
}

pub fn render_evaluation(path: &Path, name: &str, evaluation: &Evaluation) -> String {
    let mut out = String::new();
    let outcome = &evaluation.outcome;

    let _ = writeln!(out, "{} ('{name}')", path.display());
    let _ = writeln!(
        out,
        "  outcome {:?} in {:.2}% of runs, needs {:.2}%  {}",
        outcome.expected,
        outcome.observed_rate * 100.0,
        outcome.required_rate * 100.0,
        verdict(outcome.passed())
    );

    metric_row!(
        out,
        "metric",
        "observed",
        "simulated",
        "difference",
        "allowed",
        "per-battle range",
        ""
    );

    for check in &evaluation.numbers {
        metric_row!(
            out,
            check.label,
            format!("{:.3}", check.expected),
            format!("{:.3}", check.simulated),
            format!("{:.3}", check.difference()),
            format!("{:.3}", check.allowed),
            range(check),
            verdict(check.passed())
        );
    }

    out
}

/// The spread across individual battles, which is the evidence a
/// `tolerance.justification` is supposed to rest on.
fn range(check: &NumberCheck) -> String {
    if check.minimum.is_finite() && check.maximum.is_finite() {
        format!("{:.0} – {:.0}", check.minimum, check.maximum)
    } else {
        "no battles".to_owned()
    }
}

fn verdict(passed: bool) -> &'static str {
    if passed { "ok" } else { "OVER TOLERANCE" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_digits_in_threes() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn profits_carry_their_sign() {
        assert_eq!(signed_thousands(1_234), "+1,234");
        assert_eq!(signed_thousands(-1_234), "-1,234");
        assert_eq!(signed_thousands(0), "+0");
    }

    /// The entity table is what a new user reads to learn the vocabulary, so
    /// every unit has to appear in it.
    #[test]
    fn the_entity_table_lists_every_entity() {
        let table = render_entities();
        for entity in ENTITY_INFO {
            assert!(
                table.contains(entity.name),
                "{} is missing from the table",
                entity.name
            );
        }
    }

    #[test]
    fn fleets_print_in_id_order_regardless_of_hash_order() {
        let mut fleet = FleetComposition::new();
        fleet.insert(401, 20);
        fleet.insert(204, 50);
        fleet.insert(206, 100);

        let mut out = String::new();
        write_fleet(&mut out, &fleet, "");

        let order: Vec<&str> = out.lines().collect();
        assert!(order[0].starts_with("Light Fighter"), "{out}");
        assert!(order[1].starts_with("Cruiser"), "{out}");
        assert!(order[2].starts_with("Rocket Launcher"), "{out}");
    }

    #[test]
    fn an_empty_fleet_says_so_rather_than_printing_nothing() {
        let mut out = String::new();
        write_fleet(&mut out, &FleetComposition::new(), "");
        assert_eq!(out.trim(), "(none)");
    }

    /// A battle with no round detail must still print a header the user can
    /// act on, not an empty section.
    #[test]
    fn missing_round_detail_is_stated() {
        let result = SimulationResult {
            outcome: combat_types::CombatOutcome::Draw,
            rounds: 0,
            attacker_losses: FleetComposition::new(),
            defender_losses: FleetComposition::new(),
            attacker_remaining: FleetComposition::new(),
            defender_remaining: FleetComposition::new(),
            debris_field: combat_types::DebrisField::default(),
            loot: combat_types::PlanetResources::default(),
            attacker_profit: 0,
            defender_profit: 0,
            round_details: None,
            round_compositions: None,
            round_compositions_by_slot: None,
            attacker_slots: None,
            defender_slots: None,
        };
        let out = render_rounds(&result, 100);
        assert!(out.contains("simulation 1 of 100"), "{out}");
        assert!(out.contains("no round detail"), "{out}");
    }

    #[test]
    fn rounds_show_start_end_and_the_difference() {
        assert_eq!(survivors(1_500, 1_200), "1,500 -> 1,200 (-300)");
    }
}
