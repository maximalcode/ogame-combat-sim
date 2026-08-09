//! Human names and shorthand aliases for entity types.
//!
//! [`EntityStats`](crate::EntityStats) is deliberately numbers-only — it is
//! cloned per test and read in the combat loop, and nothing in combat
//! resolution needs to know that `206` is called "Cruiser". So the naming table
//! lives here instead of inside that struct: still in the shared crate, so the
//! CLI, the API and any future UI resolve the same token to the same ship, but
//! not welded to the type the engine iterates.
//!
//! The stats table has no name field, so this table is written by hand and
//! `every_entity_in_the_stats_table_has_a_name` asserts the two stay in step
//! rather than trusting that they do.
//!
//! Aliases are the abbreviations players actually type — `lf`, `ds`, `rl` — plus
//! a few German ones (`kt`, `gt`, `jf`) that are common in mixed-language
//! universes. Matching is case- and punctuation-insensitive, so `"Light
//! Fighter"`, `"light-fighter"` and `"lightfighter"` all work without being
//! listed.

use crate::EntityType;
use std::collections::HashMap;
use std::sync::LazyLock;

/// What an entity type is called, for display and for input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityInfo {
    pub entity_type: EntityType,
    /// The name `OGame` gives the unit in English.
    pub name: &'static str,
    /// Extra tokens [`resolve`] accepts. The canonical name always resolves and
    /// is not repeated here.
    pub aliases: &'static [&'static str],
}

/// Every entity, in id order.
///
/// Ordered rather than a map so `combat-cli entities` can print the table
/// without sorting it, and so ships come out before defences the way the game
/// lists them.
pub const ENTITY_INFO: &[EntityInfo] = &[
    // Ships
    info(202, "Small Cargo", &["sc", "kt"]),
    info(203, "Large Cargo", &["lc", "gt"]),
    info(204, "Light Fighter", &["lf", "jf"]),
    info(205, "Heavy Fighter", &["hf", "sf"]),
    info(206, "Cruiser", &["cr", "kr"]),
    info(207, "Battleship", &["bs", "ss"]),
    info(208, "Colony Ship", &["cs", "col"]),
    info(209, "Recycler", &["rec", "recy"]),
    info(210, "Espionage Probe", &["ep", "probe", "spy", "spio"]),
    info(211, "Bomber", &["bmb", "bomb"]),
    info(212, "Solar Satellite", &["sat", "solsat"]),
    info(213, "Destroyer", &["dest", "zer"]),
    info(214, "Deathstar", &["ds", "rip"]),
    info(215, "Battlecruiser", &["bc", "sxer"]),
    info(217, "Crawler", &["crawl"]),
    info(218, "Reaper", &["rp", "reap"]),
    info(219, "Pathfinder", &["pf", "path"]),
    // Defences
    info(401, "Rocket Launcher", &["rl", "rak"]),
    info(402, "Light Laser", &["ll"]),
    info(403, "Heavy Laser", &["hl"]),
    info(404, "Gauss Cannon", &["gc", "gauss"]),
    info(405, "Ion Cannon", &["ic", "ion"]),
    info(406, "Plasma Turret", &["pt", "plasma"]),
    info(407, "Small Shield Dome", &["ssd"]),
    info(408, "Large Shield Dome", &["lsd"]),
    // Missiles
    info(502, "Anti-Ballistic Missile", &["abm"]),
    info(503, "Interplanetary Missile", &["ipm"]),
];

const fn info(
    entity_type: EntityType,
    name: &'static str,
    aliases: &'static [&'static str],
) -> EntityInfo {
    EntityInfo {
        entity_type,
        name,
        aliases,
    }
}

/// Normalised token -> entity type, built once.
///
/// Holds both the canonical names and the aliases, so a lookup is one hash
/// rather than a scan of 27 entries times their alias lists.
static BY_TOKEN: LazyLock<HashMap<String, EntityType>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for entity in ENTITY_INFO {
        map.insert(normalise(entity.name), entity.entity_type);
        for alias in entity.aliases {
            map.insert(normalise(alias), entity.entity_type);
        }
    }
    map
});

/// Lowercase and drop everything that is not a letter or digit.
///
/// `"Light Fighter"`, `"light-fighter"` and `"LIGHTFIGHTER"` all collapse to the
/// same key, which is why the alias lists do not have to enumerate spellings.
fn normalise(token: &str) -> String {
    token
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The entry for an id, if there is one.
///
/// A linear scan of 27 entries, which is cheaper than the hash it would take to
/// avoid it. Only the token lookup — which runs against arbitrary user input
/// rather than a known id — earns a map.
fn find(entity_type: EntityType) -> Option<&'static EntityInfo> {
    ENTITY_INFO.iter().find(|e| e.entity_type == entity_type)
}

/// The display name for an entity type, or `None` if the id is not one this
/// simulator knows.
#[must_use]
pub fn name_of(entity_type: EntityType) -> Option<&'static str> {
    find(entity_type).map(|e| e.name)
}

/// Resolve a user-typed token to an entity type.
///
/// Accepts a name (`"cruiser"`, `"Light Fighter"`), an alias (`"lf"`) or the
/// numeric id (`"204"`). A numeric token that is not a known entity is `None`
/// rather than being passed through — an id the stat table has never heard of
/// would otherwise reach the engine and simply contribute nothing to the
/// battle, which looks exactly like a typo being ignored.
#[must_use]
pub fn resolve(token: &str) -> Option<EntityType> {
    let trimmed = token.trim();
    if let Ok(id) = trimmed.parse::<EntityType>() {
        return find(id).map(|e| e.entity_type);
    }
    BY_TOKEN.get(&normalise(trimmed)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::entity_stats;
    use std::collections::HashSet;

    /// The reason this module can be hand-written at all. If a ship is added to
    /// the stat table and not here, `combat-cli entities` would silently omit
    /// it and `-a "newship:10"` would report it as unknown.
    #[test]
    fn every_entity_in_the_stats_table_has_a_name() {
        let named: HashSet<EntityType> = ENTITY_INFO.iter().map(|e| e.entity_type).collect();
        let mut missing: Vec<EntityType> = entity_stats()
            .keys()
            .copied()
            .filter(|id| !named.contains(id))
            .collect();
        missing.sort_unstable();
        assert!(
            missing.is_empty(),
            "stat table entries with no name: {missing:?}"
        );
    }

    /// And the other direction: a name for an entity the engine cannot simulate
    /// would be offered by `entities` and then produce an empty fleet.
    #[test]
    fn every_named_entity_exists_in_the_stats_table() {
        let stats = entity_stats();
        let mut unknown: Vec<EntityType> = ENTITY_INFO
            .iter()
            .map(|e| e.entity_type)
            .filter(|id| !stats.contains_key(id))
            .collect();
        unknown.sort_unstable();
        assert!(
            unknown.is_empty(),
            "names with no stat table entry: {unknown:?}"
        );
    }

    /// Two entities claiming the same token would make `resolve` return
    /// whichever happened to be inserted last — a silent wrong-ship bug.
    #[test]
    fn no_token_is_claimed_by_two_entities() {
        let mut seen: HashMap<String, EntityType> = HashMap::new();
        for entity in ENTITY_INFO {
            for token in std::iter::once(entity.name).chain(entity.aliases.iter().copied()) {
                let key = normalise(token);
                if let Some(other) = seen.insert(key.clone(), entity.entity_type) {
                    assert_eq!(
                        other, entity.entity_type,
                        "token {token:?} is claimed by both {other} and {}",
                        entity.entity_type
                    );
                }
            }
        }
    }

    /// An alias that happens to be a number would shadow the numeric-id branch
    /// of `resolve` and become unreachable.
    #[test]
    fn no_alias_is_numeric() {
        for entity in ENTITY_INFO {
            for alias in entity.aliases {
                assert!(
                    alias.parse::<EntityType>().is_err(),
                    "alias {alias:?} on {} parses as an id and would be shadowed",
                    entity.entity_type
                );
            }
        }
    }

    #[test]
    fn resolves_names_aliases_and_ids_to_the_same_entity() {
        assert_eq!(resolve("Light Fighter"), Some(204));
        assert_eq!(resolve("light-fighter"), Some(204));
        assert_eq!(resolve("LIGHTFIGHTER"), Some(204));
        assert_eq!(resolve("lf"), Some(204));
        assert_eq!(resolve("204"), Some(204));
        assert_eq!(resolve("  lf  "), Some(204));
    }

    #[test]
    fn rejects_unknown_tokens_and_unknown_ids() {
        assert_eq!(resolve("stardestroyer"), None);
        assert_eq!(resolve(""), None);
        // 216 is a gap in the game's own numbering.
        assert_eq!(resolve("216"), None);
        assert_eq!(resolve("99999"), None);
    }

    #[test]
    fn names_entities_by_id() {
        assert_eq!(name_of(206), Some("Cruiser"));
        assert_eq!(name_of(216), None);
    }
}
