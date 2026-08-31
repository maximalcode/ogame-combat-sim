//! Turning command-line strings into engine types.
//!
//! Every parser here returns `Result<_, String>` and the string is the message
//! the user sees. There is no error enum because nothing branches on the kind
//! of failure — `main` prints it and exits non-zero — and an enum whose only
//! consumer is `Display` is a layer of indirection that would make the messages
//! harder to read, not easier.
//!
//! The rule the parsers share: an input we do not understand is an error, never
//! a skip. `-a "cruser:100"` typed with a missing `i` must not quietly launch an
//! empty fleet and report a crushing defeat.

use combat_types::{FleetComposition, PlanetResources, Technology, names};

/// Parse a fleet shorthand such as `"cruiser:100,lf:50"` or `"206:100"`.
///
/// Empty (or all-whitespace) input is an empty fleet, not an error: attacking
/// an undefended planet is a real scenario and `-d ""` is how you say it.
/// Repeating an entity adds the counts together, so `"lf:10,204:5"` is fifteen
/// light fighters rather than a silently dropped five.
pub fn parse_fleet(spec: &str) -> Result<FleetComposition, String> {
    let mut fleet = FleetComposition::new();

    for entry in spec.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let (token, count) = entry.split_once(':').ok_or_else(|| {
            format!("fleet entry {entry:?} has no count — write it as \"{entry}:100\"")
        })?;

        let token = token.trim();
        let entity_type = names::resolve(token).ok_or_else(|| {
            format!("unknown entity {token:?} — run `combat-cli entities` for the full list")
        })?;

        let count: u32 = count.trim().parse().map_err(|_| {
            format!(
                "count {:?} for {token:?} is not a whole number of ships",
                count.trim()
            )
        })?;

        // `+=` rather than `insert`: "lf:10,204:5" names the same ship twice and
        // means fifteen. Overwriting would drop the ten without saying so.
        *fleet.entry(entity_type).or_insert(0) += count;
    }

    Ok(fleet)
}

/// Parse technology levels: `"10/12/11"` as weapon/shield/armour, or a single
/// `"10"` meaning all three.
pub fn parse_tech(spec: &str) -> Result<Technology, String> {
    let parts: Vec<&str> = spec.split('/').map(str::trim).collect();

    let level = |raw: &str| -> Result<u8, String> {
        raw.parse::<u8>()
            .map_err(|_| format!("technology level {raw:?} is not a level between 0 and 255"))
    };

    let (weapon, shield, armour) = match parts.as_slice() {
        [all] => {
            let level = level(all)?;
            (level, level, level)
        }
        [w, s, a] => (level(w)?, level(s)?, level(a)?),
        _ => {
            return Err(format!(
                "technology {spec:?} should be \"weapon/shield/armour\" (e.g. \"10/12/11\") \
                 or one level for all three (e.g. \"10\")"
            ));
        }
    };

    Ok(Technology {
        weapon,
        shield,
        armour,
        ..Default::default()
    })
}

/// Parse `"metal,crystal,deuterium"` for the planet being attacked.
pub fn parse_resources(spec: &str) -> Result<PlanetResources, String> {
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();

    let [metal, crystal, deuterium] = parts.as_slice() else {
        return Err(format!(
            "planet resources {spec:?} should be \"metal,crystal,deuterium\" \
             (e.g. \"1000000,500000,200000\")"
        ));
    };

    let amount = |raw: &str| -> Result<u64, String> {
        raw.parse::<u64>()
            .map_err(|_| format!("resource amount {raw:?} is not a whole number"))
    };

    Ok(PlanetResources {
        metal: amount(metal)?,
        crystal: amount(crystal)?,
        deuterium: amount(deuterium)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleet(entries: &[(u16, u32)]) -> FleetComposition {
        entries.iter().copied().collect()
    }

    #[test]
    fn parses_names_aliases_and_ids() {
        assert_eq!(
            parse_fleet("cruiser:100,lf:50,401:20").expect("valid spec"),
            fleet(&[(206, 100), (204, 50), (401, 20)])
        );
    }

    #[test]
    fn tolerates_spaces_around_separators() {
        assert_eq!(
            parse_fleet(" cruiser : 100 , lf:50 ").expect("valid spec"),
            fleet(&[(206, 100), (204, 50)])
        );
    }

    #[test]
    fn an_empty_spec_is_an_empty_fleet() {
        assert!(parse_fleet("").expect("empty is valid").is_empty());
        assert!(parse_fleet("   ").expect("whitespace is valid").is_empty());
    }

    #[test]
    fn repeating_an_entity_sums_the_counts() {
        assert_eq!(
            parse_fleet("lf:10,204:5").expect("valid spec"),
            fleet(&[(204, 15)])
        );
    }

    /// The acceptance criterion this crate exists to satisfy: a typo must be
    /// loud. Silently dropping the token turns a misspelled fleet into a lost
    /// battle that looks like a simulation result.
    #[test]
    fn an_unknown_name_names_itself_in_the_error() {
        let err = parse_fleet("cruser:100").expect_err("typo must not parse");
        assert!(
            err.contains("cruser"),
            "message should quote the token: {err}"
        );
    }

    #[test]
    fn an_unknown_id_is_rejected_too() {
        let err = parse_fleet("216:100").expect_err("216 is not an entity");
        assert!(err.contains("216"), "message should quote the token: {err}");
    }

    #[test]
    fn a_missing_count_is_an_error() {
        let err = parse_fleet("cruiser").expect_err("no count");
        assert!(
            err.contains("cruiser"),
            "message should quote the entry: {err}"
        );
    }

    #[test]
    fn a_non_numeric_count_is_an_error() {
        let err = parse_fleet("cruiser:many").expect_err("count is not a number");
        assert!(
            err.contains("many"),
            "message should quote the count: {err}"
        );
    }

    #[test]
    fn a_negative_count_is_an_error() {
        let err = parse_fleet("cruiser:-5").expect_err("negative count");
        assert!(err.contains("-5"), "message should quote the count: {err}");
    }

    #[test]
    fn parses_three_tech_levels() {
        assert_eq!(
            parse_tech("10/12/11").expect("valid tech"),
            Technology {
                weapon: 10,
                shield: 12,
                armour: 11,
                ..Default::default()
            }
        );
    }

    #[test]
    fn a_single_tech_level_applies_to_all_three() {
        assert_eq!(
            parse_tech("14").expect("valid tech"),
            Technology {
                weapon: 14,
                shield: 14,
                armour: 14,
                ..Default::default()
            }
        );
    }

    #[test]
    fn tech_rejects_the_wrong_number_of_parts() {
        assert!(parse_tech("10/10").is_err());
        assert!(parse_tech("10/10/10/10").is_err());
    }

    #[test]
    fn tech_rejects_levels_out_of_range() {
        // Levels are u8 in the engine; 300 is not a level anyone has.
        let err = parse_tech("300/10/10").expect_err("out of range");
        assert!(err.contains("300"), "message should quote the level: {err}");
    }

    #[test]
    fn parses_planet_resources() {
        assert_eq!(
            parse_resources("1000000,500000,200000").expect("valid resources"),
            PlanetResources {
                metal: 1_000_000,
                crystal: 500_000,
                deuterium: 200_000,
            }
        );
    }

    #[test]
    fn resources_reject_the_wrong_number_of_parts() {
        assert!(parse_resources("1000,2000").is_err());
        assert!(parse_resources("1000,2000,3000,4000").is_err());
    }
}
