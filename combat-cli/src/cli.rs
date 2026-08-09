//! The command line surface, and the translation from it into a
//! [`CombatRequest`].
//!
//! Two subcommands: `sim` runs a battle, `entities` prints the stat table.
//!
//! `sim` takes a battle either as flags or as a JSON file. The JSON path is a
//! straight `serde_json::from_str::<CombatRequest>` — the same body
//! `POST /api/simulate` accepts, with no parallel schema to keep in step.
//!
//! **The CLI does not inherit the server's limits.** `combat-api` caps
//! `simulations` and forces downscaling to auto; those exist so one HTTP caller
//! cannot monopolise a shared process. A local binary spending its own CPU has
//! no such problem, so `-n 100000` runs a hundred thousand battles and
//! `--downscaling off` really turns it off.

use clap::{Args, Parser, Subcommand, ValueEnum};
use combat_types::{CombatRequest, PartyData, Technology};

use crate::args::{parse_fleet, parse_resources, parse_tech};

#[derive(Debug, Parser)]
#[command(
    name = "combat-cli",
    version,
    about = "Simulate OGame fleet combat from the command line",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run a battle and print a combat report.
    Sim(Box<SimArgs>),
    /// List every entity the simulator knows: id, name, aliases and stats.
    Entities,
}

/// How downscaling is decided.
///
/// Mirrors `CombatRequest::enable_downscaling`, which is an `Option<bool>` with
/// `None` meaning "decide from the fleet size". A tri-state flag reads better on
/// a command line than a bare `--downscaling true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Downscaling {
    /// Approximate only above the engine's threshold (10M ships).
    Auto,
    /// Always approximate.
    On,
    /// Never approximate, however long it takes.
    Off,
}

impl From<Downscaling> for Option<bool> {
    fn from(value: Downscaling) -> Self {
        match value {
            Downscaling::Auto => None,
            Downscaling::On => Some(true),
            Downscaling::Off => Some(false),
        }
    }
}

#[derive(Debug, Args)]
pub struct SimArgs {
    /// Attacking fleet, e.g. "cruiser:100,lf:50" or "206:100".
    #[arg(short = 'a', long, default_value = "", conflicts_with = "file")]
    pub attacker: String,

    /// Defending fleet and defences, e.g. "lf:1000,rocketlauncher:200".
    #[arg(short = 'd', long, default_value = "", conflicts_with = "file")]
    pub defender: String,

    /// Weapon/shield/armour levels for both sides, e.g. "10/12/11" or "10".
    #[arg(long, default_value = "0", conflicts_with = "file")]
    pub tech: String,

    /// Attacker technology, overriding --tech.
    #[arg(long, conflicts_with = "file")]
    pub attacker_tech: Option<String>,

    /// Defender technology, overriding --tech.
    #[arg(long, conflicts_with = "file")]
    pub defender_tech: Option<String>,

    // The defaults below are read off `CombatRequest::default()` rather than
    // written out, so the flags and the library cannot disagree about what
    // "unset" means. Kept as `//` and not `///` because clap renders doc
    // comments into `--help`, where our internal reasoning has no business.
    /// Battles to run. Not capped: this is your CPU, not a shared server.
    #[arg(short = 'n', long, default_value_t = CombatRequest::default().simulations, conflicts_with = "file")]
    pub simulations: u32,

    /// Turn rapid fire off. It is on in game, so it is on by default here.
    #[arg(long, conflicts_with = "file")]
    pub no_rapid_fire: bool,

    /// Percentage of destroyed ships that becomes debris.
    #[arg(
        long,
        value_name = "PERCENT",
        default_value_t = CombatRequest::default().debris_percentage,
        value_parser = percentage,
        conflicts_with = "file",
    )]
    pub debris: f32,

    /// Percentage of the planet's resources a winning attacker takes.
    ///
    /// In game this is 50, 75 or 100 depending on the attacker's class; any
    /// value up to 100 is accepted here because the engine takes one.
    #[arg(
        long,
        value_name = "PERCENT",
        default_value_t = CombatRequest::default().plunder_percentage,
        value_parser = clap::value_parser!(u8).range(0..=100),
        conflicts_with = "file",
    )]
    pub plunder: u8,

    /// Resources on the planet, "metal,crystal,deuterium". Without it there is
    /// nothing to loot.
    #[arg(long, value_name = "M,C,D", conflicts_with = "file")]
    pub planet: Option<String>,

    /// Whether to approximate very large battles.
    #[arg(long, value_enum, default_value_t = Downscaling::Auto, conflicts_with = "file")]
    pub downscaling: Downscaling,

    /// Print the round-by-round breakdown of one representative battle.
    #[arg(long)]
    pub rounds: bool,

    /// Read the battle from a JSON file instead — the same body
    /// POST /api/simulate accepts.
    #[arg(short = 'f', long, value_name = "PATH")]
    pub file: Option<std::path::PathBuf>,
}

/// `clap`'s numeric ranges only cover integers, so the one float flag needs its
/// own parser. Without it `--debris 900` is accepted and nine times the wreck is
/// salvaged — invalid input that exits zero.
fn percentage(raw: &str) -> Result<f32, String> {
    let value: f32 = raw
        .parse()
        .map_err(|_| format!("{raw:?} is not a number"))?;
    if (0.0..=100.0).contains(&value) {
        Ok(value)
    } else {
        Err(format!("{value} is not a percentage between 0 and 100"))
    }
}

/// One side of the battle, so the two sides cannot be assembled differently.
///
/// `label` is the flag name the error should blame — `"attacker"` produces
/// `--attacker:` and `--attacker-tech:` — because a message that says only
/// "unknown entity" leaves the user reading both halves of their command.
fn party(
    label: &str,
    fleet: &str,
    tech: Option<&str>,
    shared: Technology,
) -> Result<PartyData, String> {
    let entities = parse_fleet(fleet).map_err(|e| format!("--{label}: {e}"))?;
    let technology = match tech {
        Some(spec) => parse_tech(spec).map_err(|e| format!("--{label}-tech: {e}"))?,
        None => shared,
    };
    Ok(PartyData {
        technology,
        entities,
    })
}

/// Build a request from the flags.
///
/// Not used for `--file`; that path goes through [`parse_request_json`].
pub fn build_request(args: &SimArgs) -> Result<CombatRequest, String> {
    let shared_tech = parse_tech(&args.tech).map_err(|e| format!("--tech: {e}"))?;

    let attacker = party(
        "attacker",
        &args.attacker,
        args.attacker_tech.as_deref(),
        shared_tech,
    )?;
    let defender = party(
        "defender",
        &args.defender,
        args.defender_tech.as_deref(),
        shared_tech,
    )?;

    let planet_resources = args
        .planet
        .as_deref()
        .map(parse_resources)
        .transpose()
        .map_err(|e| format!("--planet: {e}"))?;

    // `..Default::default()` covers the fields with no flag — the slot lists,
    // the round-composition switch, `universe_settings`, which nothing reads
    // yet, and the two bonus blocks, which the engine does read. Leaving those
    // `None` fights a classless battle, which is what a shorthand invocation
    // means; `--file` is how a General or a Warrior alliance gets described. It
    // is only safe because `CombatRequest`'s `Default` is hand-written to agree
    // with the serde defaults; a derived one would set `debris_percentage` to
    // 0.0 here.
    Ok(CombatRequest {
        attacker,
        defender,
        planet_resources,
        debris_percentage: args.debris,
        use_rapid_fire: !args.no_rapid_fire,
        simulations: args.simulations,
        enable_downscaling: args.downscaling.into(),
        plunder_percentage: args.plunder,
        ..Default::default()
    })
}

/// Deserialize a request from the JSON body the API accepts.
///
/// Goes through `serde_path_to_error` rather than `serde_json::from_str` so a
/// wrong-typed value reports which field it was. Plain serde says
/// `invalid type: string "lots", expected u32 at line 1 column 114` and leaves
/// the user counting columns; this says `attacker.entities.206` as well.
pub fn parse_request_json(json: &str) -> Result<CombatRequest, String> {
    let deserializer = &mut serde_json::Deserializer::from_str(json);

    serde_path_to_error::deserialize(deserializer).map_err(|error| {
        let path = error.path().to_string();
        let inner = error.into_inner();
        // The path is "." when the failure is not inside any field — malformed
        // JSON, or a body that is not an object at all.
        if path == "." {
            format!("invalid combat request JSON: {inner}")
        } else {
            format!("invalid combat request JSON at {path}: {inner}")
        }
    })
}

/// Reject a request that cannot produce a report, with a message naming the
/// field at fault.
///
/// Shared by both paths, so a JSON file gets the same guard as the flags. Both
/// cases are ones the engine cannot answer rather than ones we would rather it
/// did not: an empty battle has no report, and averaging zero simulations
/// panics in the report builder.
pub fn validate(request: &CombatRequest) -> Result<(), String> {
    let attackers: u32 = request.attacker.entities.values().sum();
    let defenders: u32 = request.defender.entities.values().sum();

    if attackers == 0 && defenders == 0 {
        return Err(
            "both fleets are empty; there is nothing to simulate — give at least one of \
             --attacker or --defender"
                .to_owned(),
        );
    }
    if request.simulations == 0 {
        return Err("simulations must be at least 1".to_owned());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse an argv the way the real binary does, then build the request. This
    /// exercises the clap wiring and the translation together — testing
    /// `build_request` against a hand-built `SimArgs` would prove the flags are
    /// spelled correctly only in the test.
    fn request_from(argv: &[&str]) -> Result<CombatRequest, String> {
        let cli = Cli::try_parse_from(argv).map_err(|e| e.to_string())?;
        match cli.command {
            Command::Sim(args) => build_request(&args),
            Command::Entities => panic!("test argv should be a sim"),
        }
    }

    #[test]
    fn builds_both_fleets_from_shorthand() {
        let request =
            request_from(&["combat-cli", "sim", "-a", "cruiser:100", "-d", "lf:1000"]).unwrap();
        assert_eq!(request.attacker.entities.get(&206), Some(&100));
        assert_eq!(request.defender.entities.get(&204), Some(&1000));
    }

    #[test]
    fn tech_applies_to_both_sides() {
        let request =
            request_from(&["combat-cli", "sim", "-a", "cruiser:1", "--tech", "10/12/11"]).unwrap();
        assert_eq!(request.attacker.technology, parse_tech("10/12/11").unwrap());
        assert_eq!(request.defender.technology, parse_tech("10/12/11").unwrap());
    }

    #[test]
    fn per_side_tech_overrides_the_shared_flag() {
        let request = request_from(&[
            "combat-cli",
            "sim",
            "-a",
            "cruiser:1",
            "--tech",
            "10",
            "--attacker-tech",
            "15",
        ])
        .unwrap();
        assert_eq!(request.attacker.technology.weapon, 15);
        assert_eq!(request.defender.technology.weapon, 10);
    }

    /// The brief is explicit about this: the API's cap is server protection,
    /// not an engine rule, and copying it into the CLI would be cargo-culting
    /// a limit that has no reason to exist locally.
    #[test]
    fn simulations_are_not_capped_at_the_api_limit() {
        let request =
            request_from(&["combat-cli", "sim", "-a", "cruiser:1", "-n", "50000"]).unwrap();
        assert_eq!(request.simulations, 50_000);
    }

    #[test]
    fn rapid_fire_is_on_unless_turned_off() {
        assert!(
            request_from(&["combat-cli", "sim", "-a", "cruiser:1"])
                .unwrap()
                .use_rapid_fire
        );
        assert!(
            !request_from(&["combat-cli", "sim", "-a", "cruiser:1", "--no-rapid-fire"])
                .unwrap()
                .use_rapid_fire
        );
    }

    #[test]
    fn downscaling_maps_onto_the_engines_tri_state() {
        let of = |flag: &str| {
            request_from(&[
                "combat-cli",
                "sim",
                "-a",
                "cruiser:1",
                "--downscaling",
                flag,
            ])
            .unwrap()
            .enable_downscaling
        };
        assert_eq!(of("auto"), None);
        assert_eq!(of("on"), Some(true));
        assert_eq!(of("off"), Some(false));
        assert_eq!(
            request_from(&["combat-cli", "sim", "-a", "cruiser:1"])
                .unwrap()
                .enable_downscaling,
            None,
            "no flag should mean auto"
        );
    }

    #[test]
    fn planet_resources_are_optional() {
        assert!(
            request_from(&["combat-cli", "sim", "-a", "cruiser:1"])
                .unwrap()
                .planet_resources
                .is_none()
        );

        let request = request_from(&[
            "combat-cli",
            "sim",
            "-a",
            "cruiser:1",
            "--planet",
            "1000,2000,3000",
        ])
        .unwrap();
        assert_eq!(
            request.planet_resources.unwrap(),
            parse_resources("1000,2000,3000").unwrap()
        );
    }

    /// Defaults that are not the flags' business: they belong to
    /// `CombatRequest`, and duplicating the numbers here is how the two drift.
    #[test]
    fn unset_economics_match_the_request_defaults() {
        let request = request_from(&["combat-cli", "sim", "-a", "cruiser:1"]).unwrap();
        let default = CombatRequest::default();
        assert!((request.debris_percentage - default.debris_percentage).abs() < f32::EPSILON);
        assert_eq!(request.plunder_percentage, default.plunder_percentage);
    }

    #[test]
    fn a_bad_fleet_spec_names_the_side_it_came_from() {
        let err = request_from(&["combat-cli", "sim", "-a", "cruser:100"]).unwrap_err();
        assert!(err.contains("attacker"), "should name the flag: {err}");
        assert!(err.contains("cruser"), "should quote the token: {err}");
    }

    #[test]
    fn a_bad_tech_spec_names_the_side_it_came_from() {
        let err = request_from(&[
            "combat-cli",
            "sim",
            "-a",
            "cruiser:1",
            "--defender-tech",
            "x",
        ])
        .unwrap_err();
        assert!(err.contains("defender-tech"), "should name the flag: {err}");
    }

    /// Percentages are the one place the flags can produce a request the engine
    /// will happily run and no player could ever face — 200% plunder takes twice
    /// what the planet holds. clap rejects them before `build_request` sees them,
    /// so these assert on the parse rather than the result.
    #[test]
    fn percentages_above_a_hundred_are_rejected() {
        assert!(
            request_from(&["combat-cli", "sim", "-a", "cruiser:1", "--plunder", "200"]).is_err()
        );
        assert!(
            request_from(&["combat-cli", "sim", "-a", "cruiser:1", "--debris", "900"]).is_err()
        );
        assert!(request_from(&["combat-cli", "sim", "-a", "cruiser:1", "--debris", "-1"]).is_err());
    }

    #[test]
    fn percentages_within_range_are_accepted() {
        let request =
            request_from(&["combat-cli", "sim", "-a", "cruiser:1", "--plunder", "75"]).unwrap();
        assert_eq!(request.plunder_percentage, 75);
    }

    #[test]
    fn two_empty_fleets_are_rejected() {
        let err = validate(&CombatRequest::default()).unwrap_err();
        assert!(err.contains("empty"), "should say what is wrong: {err}");
    }

    #[test]
    fn zero_simulations_are_rejected() {
        let request = CombatRequest {
            attacker: PartyData {
                entities: parse_fleet("cruiser:1").unwrap(),
                ..Default::default()
            },
            simulations: 0,
            ..Default::default()
        };
        let err = validate(&request).unwrap_err();
        assert!(err.contains("simulations"), "should name the field: {err}");
    }

    /// The acceptance criterion for `--file`: a body the API accepts must be
    /// accepted here unchanged. Asserting against the literal rather than a
    /// round-trip of our own serialisation is the point — it is the API's
    /// wire format that has to keep working.
    #[test]
    fn accepts_the_api_request_body_unchanged() {
        let body = r#"{
            "attacker": {
                "technology": { "weapon": 12, "shield": 12, "armour": 12 },
                "entities": { "206": 100, "204": 50 }
            },
            "defender": {
                "technology": { "weapon": 10, "shield": 10, "armour": 10 },
                "entities": { "204": 1000, "401": 200 }
            },
            "use_rapid_fire": true,
            "simulations": 250,
            "planet_resources": { "metal": 500000, "crystal": 250000, "deuterium": 100000 },
            "debris_percentage": 30.0,
            "plunder_percentage": 50
        }"#;

        let request = parse_request_json(body).expect("API body should deserialize");
        assert_eq!(request.attacker.entities.get(&206), Some(&100));
        assert_eq!(request.defender.entities.get(&401), Some(&200));
        assert_eq!(request.simulations, 250);
        assert_eq!(request.attacker.technology.weapon, 12);
        assert!(validate(&request).is_ok());
    }

    #[test]
    fn a_minimal_json_body_is_enough() {
        let request = parse_request_json(
            r#"{ "attacker": { "technology": {}, "entities": { "206": 1 } },
                 "defender": { "technology": {}, "entities": {} },
                 "use_rapid_fire": true, "simulations": 10 }"#,
        )
        .expect("minimal body should deserialize");
        assert!(validate(&request).is_ok());
    }

    /// "Invalid input exits non-zero with a message naming the offending
    /// field" has to hold for `--file` too, and serde on its own only offers a
    /// line and column.
    #[test]
    fn a_wrong_typed_json_field_is_named() {
        let err = parse_request_json(
            r#"{ "attacker": { "technology": {}, "entities": { "206": "lots" } },
                 "defender": { "technology": {}, "entities": {} },
                 "use_rapid_fire": true, "simulations": 10 }"#,
        )
        .unwrap_err();
        assert!(
            err.contains("attacker.entities"),
            "should name the field: {err}"
        );
    }

    #[test]
    fn malformed_json_reports_where_it_broke() {
        let err = parse_request_json("{ not json").unwrap_err();
        assert!(
            err.contains("line") || err.contains("column"),
            "serde's position should survive: {err}"
        );
    }

    /// `--file` describes the whole battle, so combining it with fleet flags is
    /// a request with two answers. clap rejects it before we ever look.
    #[test]
    fn file_and_fleet_flags_are_mutually_exclusive() {
        assert!(
            Cli::try_parse_from(["combat-cli", "sim", "-f", "b.json", "-a", "cruiser:1"]).is_err()
        );
    }
}
