use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Composition = BTreeMap<u16, u32>;

/// Sanitized review artifact, never implicitly converted into a simulation request.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Candidate {
    pub schema_version: u8,
    pub report_kind: super::ReportKind,
    pub provenance: Provenance,
    pub attackers: Vec<Participant>,
    pub defenders: Vec<Participant>,
    pub observed: Option<serde_json::Value>,
    pub planet_resources: Option<ResourcesCandidate>,
    pub loot_percentage: Option<u8>,
    pub review_required: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourcesCandidate {
    pub metal: Option<u64>,
    pub crystal: Option<u64>,
    pub deuterium: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Provenance {
    pub source: String,
    pub community: String,
    pub universe: u32,
    pub event_timestamp: Option<u64>,
    pub game_version: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Participant {
    pub slot: String,
    pub entities: Option<Composition>,
    pub ships: Option<Composition>,
    pub defenses: Option<Composition>,
    pub technology: TechnologyCandidate,
    pub character_class_id: Option<u8>,
    pub alliance_class_id: Option<u8>,
    /// Proxy coefficients, NOT simulator lifeform percentages. Their treatment
    /// differs between report variants; review against reported unit stats.
    pub reported_base_stats_booster: Option<serde_json::Value>,
    pub reported_unit_stats: Option<serde_json::Value>,
}

/// Unknown levels are null, never zero. Basis distinguishes research from
/// combat-reported bonuses, whose class treatment needs explicit review.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TechnologyCandidate {
    pub basis: String,
    pub weapon: Option<u8>,
    pub shield: Option<u8>,
    pub armour: Option<u8>,
}
