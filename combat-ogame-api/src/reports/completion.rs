//! Offline completion of one sanitized combat report.
//!
//! This module is deliberately downstream of parsing.  A parsed candidate is
//! evidence, not a request; callers must explicitly provide the facts whose
//! basis the provider did not establish and pin the universe snapshot before
//! a [`CombatRequest`] is produced.

use super::ReportKind;
use super::model::{Candidate, Composition, Participant};
use combat_types::entities::entity_stats;
use combat_types::{
    AllianceClass, CombatRequest, EntityStats, LifeformBonus, LifeformBonuses, PartyData,
    PlayerBonuses, PlayerClass, Technology, UniverseSettings,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Where an accepted value came from.  These values are intentionally kept
/// separate even when they happen to contain the same number.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    Report,
    PublicMetadata,
    Supplied,
}

/// A single accepted value in the evidence ledger.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct EvidenceRecord {
    pub source: EvidenceSource,
    pub value: Value,
}

/// Evidence retained separately from the simulator request.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct EvidenceLedger {
    pub fields: BTreeMap<String, EvidenceRecord>,
}

impl EvidenceLedger {
    fn report(&mut self, location: impl Into<String>, value: Value) {
        self.fields.insert(
            location.into(),
            EvidenceRecord {
                source: EvidenceSource::Report,
                value,
            },
        );
    }

    fn supplied(&mut self, location: impl Into<String>, value: Value) {
        self.fields.insert(
            location.into(),
            EvidenceRecord {
                source: EvidenceSource::Supplied,
                value,
            },
        );
    }

    fn public_metadata(&mut self, location: impl Into<String>, value: Value) {
        self.fields.insert(
            location.into(),
            EvidenceRecord {
                source: EvidenceSource::PublicMetadata,
                value,
            },
        );
    }
}

/// Basis of technology values supplied for completion.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TechnologyBasis {
    Researched,
    AlreadyEffective,
}

/// Explicit technology evidence for one participant.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TechnologyEvidence {
    pub basis: TechnologyBasis,
    pub weapon: u8,
    pub shield: u8,
    pub armour: u8,
}

/// Additional facts supplied for one report participant.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct ParticipantEvidence {
    #[serde(default)]
    pub entities: Option<Composition>,
    #[serde(default)]
    pub technology: Option<TechnologyEvidence>,
    #[serde(default)]
    pub player_class: Option<PlayerClass>,
    #[serde(default)]
    pub alliance_class: Option<AllianceClass>,
    /// Percentages are simulator units: `50.0` means +50%.
    #[serde(default)]
    pub lifeform: BTreeMap<u16, LifeformBonus>,
}

/// Evidence used to complete the candidate.  The map is keyed by the stable
/// candidate slots (`A1`, `D1`), so no provider owner id is needed.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct CompletionEvidence {
    #[serde(default)]
    pub participants: BTreeMap<String, ParticipantEvidence>,
}

/// Every universe field is optional at the artifact boundary.  This prevents
/// serde's structural defaults from turning a partial public response into a
/// complete historical universe.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct PinnedUniverseSettings {
    pub galaxies: Option<u8>,
    pub systems: Option<u16>,
    pub donut_galaxy: Option<bool>,
    pub donut_systems: Option<bool>,
    pub fleet_speed: Option<u8>,
    pub debris_fleet: Option<u8>,
    pub debris_defence: Option<u8>,
    pub debris_deuterium: Option<bool>,
    pub deuterium_save_factor: Option<u8>,
}

impl PinnedUniverseSettings {
    fn resolve(&self) -> Result<UniverseSettings, &'static str> {
        let settings = UniverseSettings {
            galaxies: self.galaxies.ok_or("galaxies")?,
            systems: self.systems.ok_or("systems")?,
            donut_galaxy: self.donut_galaxy.ok_or("donut_galaxy")?,
            donut_systems: self.donut_systems.ok_or("donut_systems")?,
            fleet_speed: self.fleet_speed.ok_or("fleet_speed")?,
            debris_fleet: self.debris_fleet.ok_or("debris_fleet")?,
            debris_defence: self.debris_defence.ok_or("debris_defence")?,
            debris_deuterium: self.debris_deuterium.ok_or("debris_deuterium")?,
            deuterium_save_factor: self.deuterium_save_factor.ok_or("deuterium_save_factor")?,
        };
        if !(1..=9).contains(&settings.galaxies)
            || !(1..=499).contains(&settings.systems)
            || settings.fleet_speed == 0
            || settings.debris_fleet > 100
            || settings.debris_defence > 100
            || settings.deuterium_save_factor > 100
        {
            return Err("values");
        }
        Ok(settings)
    }
}

/// A complete, explicitly pinned universe snapshot supplied by the caller.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PinnedUniverse {
    pub community: String,
    pub universe: u32,
    pub settings: PinnedUniverseSettings,
    pub source: EvidenceSource,
    pub source_timestamp: Option<u64>,
    pub source_version: Option<String>,
    #[serde(default)]
    pub current: bool,
    #[serde(default)]
    pub acknowledged_current: bool,
}

/// Structured local artifact consumed by both the library and CLI.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompletionInput {
    pub candidate: Candidate,
    #[serde(default)]
    pub evidence: CompletionEvidence,
    pub universe: PinnedUniverse,
}

impl CompletionInput {
    /// Run the offline completion workflow for this local artifact.
    #[must_use]
    pub fn complete(&self) -> CompletionResult {
        complete_candidate(self)
    }
}

/// Stable issue categories returned by completion.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldIssueKind {
    Unknown,
    Hidden,
    Missing,
    Contradictory,
    UnsupportedBasis,
    ReportStatMismatch,
    WrongUniverse,
    IncompleteUniverse,
    CurrentSnapshotUnacknowledged,
    Unsupported,
}

/// One actionable completion problem. All independent issues are returned.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldIssue {
    pub kind: FieldIssueKind,
    pub location: String,
    pub explanation: String,
    pub evidence_requests: Vec<String>,
}

/// A verified request and its evidence ledger. The observed report is kept out
/// of `request` so it cannot accidentally become simulation input.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VerifiedBattleInput {
    pub request: CombatRequest,
    pub evidence: EvidenceLedger,
    pub observed: Option<Value>,
}

/// Completion always has one of these two machine-readable outcomes.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CompletionResult {
    Verified { input: Box<VerifiedBattleInput> },
    Incomplete { issues: Vec<FieldIssue> },
}

/// Complete one combat candidate entirely offline.
#[must_use]
pub fn complete_candidate(input: &CompletionInput) -> CompletionResult {
    let candidate = &input.candidate;
    let mut issues = Vec::new();
    let mut evidence = EvidenceLedger::default();

    if candidate.report_kind != ReportKind::Combat {
        issue(
            &mut issues,
            FieldIssueKind::Unsupported,
            "report_kind",
            "only a single combat report can be completed by this workflow",
            "supply a combat report candidate",
        );
    }
    if candidate.attackers.len() != 1 {
        issue(
            &mut issues,
            if candidate.attackers.is_empty() {
                FieldIssueKind::Missing
            } else {
                FieldIssueKind::Unsupported
            },
            "attackers",
            "completion requires exactly one attacker",
            "supply one attacker participant",
        );
    }
    if candidate.defenders.len() != 1 {
        issue(
            &mut issues,
            if candidate.defenders.is_empty() {
                FieldIssueKind::Missing
            } else {
                FieldIssueKind::Unsupported
            },
            "defenders",
            "completion requires exactly one defender",
            "supply one defender participant",
        );
    }
    validate_universe(candidate, &input.universe, &mut issues, &mut evidence);

    let attacker = candidate.attackers.first().map(|p| {
        resolve_participant(
            p,
            &input.evidence,
            &candidate.review_required,
            &mut issues,
            &mut evidence,
        )
    });
    let defender = candidate.defenders.first().map(|p| {
        resolve_participant(
            p,
            &input.evidence,
            &candidate.review_required,
            &mut issues,
            &mut evidence,
        )
    });

    if !issues.is_empty() {
        return CompletionResult::Incomplete { issues };
    }
    let (Some(attacker), Some(defender), Ok(settings)) =
        (attacker, defender, input.universe.settings.resolve())
    else {
        return CompletionResult::Incomplete { issues };
    };

    let request = CombatRequest {
        attacker: attacker.party,
        defender: defender.party,
        universe_settings: Some(settings),
        simulations: 1,
        planet_resources: None,
        plunder_percentage: candidate.loot_percentage.unwrap_or(50),
        // Per-participant technology is already effective.  Keeping these
        // blocks empty is what prevents class bonuses being applied twice.
        attacker_bonuses: None,
        defender_bonuses: None,
        ..CombatRequest::default()
    };
    evidence.public_metadata(
        "universe",
        serde_json::to_value(&input.universe).unwrap_or(Value::Null),
    );
    CompletionResult::Verified {
        input: Box::new(VerifiedBattleInput {
            request,
            evidence,
            observed: candidate.observed.clone(),
        }),
    }
}

/// Alias spelling used by callers that describe the operation as a workflow.
#[must_use]
pub fn complete_report(input: &CompletionInput) -> CompletionResult {
    complete_candidate(input)
}

struct ResolvedParticipant {
    party: PartyData,
}

#[allow(clippy::too_many_lines)]
fn resolve_participant(
    participant: &Participant,
    all_evidence: &CompletionEvidence,
    review_required: &[String],
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) -> ResolvedParticipant {
    let location = participant.slot.clone();
    let supplied = all_evidence.participants.get(&location);
    let entities = match (
        &participant.entities,
        supplied.and_then(|e| e.entities.as_ref()),
    ) {
        (Some(observed), Some(provided)) if observed != provided => {
            issue(
                issues,
                FieldIssueKind::Contradictory,
                format!("{location}.entities"),
                "supplied composition conflicts with the observed report composition",
                "correct the supplied composition or provide a matching report",
            );
            None
        }
        (Some(observed), _) => {
            ledger.report(format!("{location}.entities"), json_composition(observed));
            Some(observed.clone())
        }
        (None, Some(provided)) => {
            ledger.supplied(format!("{location}.entities"), json_composition(provided));
            Some(provided.clone())
        }
        (None, None) => {
            let kind = review_kind(review_required, &location, "entities");
            issue(
                issues,
                kind,
                format!("{location}.entities"),
                "the report does not contain an accepted fleet composition",
                "supply the exact participant composition from the report",
            );
            None
        }
    };

    let player_class = resolve_class(
        participant.character_class_id.map(player_class),
        supplied.and_then(|e| e.player_class),
        &location,
        "player_class",
        issues,
        ledger,
    );
    let alliance_class = resolve_class(
        participant.alliance_class_id.map(alliance_class),
        supplied.and_then(|e| e.alliance_class),
        &location,
        "alliance_class",
        issues,
        ledger,
    );
    let technology = resolve_technology(
        participant,
        supplied,
        player_class.unwrap_or_default(),
        alliance_class.unwrap_or_default(),
        review_required,
        issues,
        ledger,
    );
    let lifeform = supplied.map_or_else(LifeformBonuses::default, |e| {
        for (&entity, bonus) in &e.lifeform {
            ledger.supplied(
                format!("{location}.lifeform.{entity}"),
                serde_json::to_value(bonus).unwrap_or(Value::Null),
            );
        }
        e.lifeform
            .iter()
            .map(|(&entity, &bonus)| (entity, bonus))
            .collect()
    });

    if let Some(stats) = participant.reported_unit_stats.as_ref() {
        ledger.report(format!("{location}.reported_unit_stats"), stats.clone());
    }
    if let Some(boosters) = participant.reported_base_stats_booster.as_ref() {
        ledger.report(
            format!("{location}.reported_base_stats_booster"),
            boosters.clone(),
        );
    }

    if let (Some(composition), Some(technology)) = (
        participant
            .entities
            .as_ref()
            .or_else(|| supplied.and_then(|e| e.entities.as_ref())),
        technology.as_ref(),
    ) {
        validate_reported_stats(participant, composition, technology, &lifeform, issues);
    }

    let party = match (entities, technology) {
        (Some(entities), Some(technology)) => Some(PartyData {
            entities: entities.into_iter().collect(),
            technology,
            lifeform,
        }),
        _ => None,
    };

    ResolvedParticipant {
        party: party.unwrap_or_default(),
    }
}

fn resolve_technology(
    participant: &Participant,
    supplied: Option<&ParticipantEvidence>,
    player_class: PlayerClass,
    alliance_class: AllianceClass,
    review_required: &[String],
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) -> Option<Technology> {
    let location = format!("{}.technology", participant.slot);
    let supplied_technology = supplied.and_then(|e| e.technology.as_ref());
    let candidate_technology = &participant.technology;
    if candidate_technology.weapon.is_some()
        || candidate_technology.shield.is_some()
        || candidate_technology.armour.is_some()
    {
        ledger.report(
            format!("{location}.reported"),
            serde_json::to_value(candidate_technology).unwrap_or(Value::Null),
        );
    }
    let Some(explicit) = supplied_technology else {
        if candidate_technology.weapon.is_some()
            || candidate_technology.shield.is_some()
            || candidate_technology.armour.is_some()
        {
            issue(
                issues,
                FieldIssueKind::UnsupportedBasis,
                location,
                "provider combat percentages are provisional and do not establish researched versus already-effective technology",
                "supply all three levels and identify their basis as researched or already effective",
            );
        } else {
            issue(
                issues,
                review_kind(review_required, &participant.slot, "technology"),
                location,
                "weapon, shield and armour levels are required",
                "supply all three combat technology levels and their basis",
            );
        }
        return None;
    };
    let values = Technology {
        weapon: explicit.weapon,
        shield: explicit.shield,
        armour: explicit.armour,
        ..Technology::default()
    };
    ledger.supplied(
        format!("{}.technology", participant.slot),
        serde_json::to_value(explicit).unwrap_or(Value::Null),
    );
    let bonuses = PlayerBonuses {
        player_class,
        alliance_class,
        ..PlayerBonuses::default()
    };
    let effective = match explicit.basis {
        TechnologyBasis::Researched => values.effective_levels(Some(&bonuses)),
        TechnologyBasis::AlreadyEffective => values,
    };
    if let Some(candidate_value) = candidate_technology.weapon {
        if candidate_value != effective.weapon {
            contradiction(issues, &format!("{}.weapon", participant.slot));
        }
    }
    if let Some(candidate_value) = candidate_technology.shield {
        if candidate_value != effective.shield {
            contradiction(issues, &format!("{}.shield", participant.slot));
        }
    }
    if let Some(candidate_value) = candidate_technology.armour {
        if candidate_value != effective.armour {
            contradiction(issues, &format!("{}.armour", participant.slot));
        }
    }
    Some(effective)
}

fn resolve_class<T: Copy + PartialEq + Serialize>(
    observed: Option<T>,
    supplied: Option<T>,
    location: &str,
    field: &str,
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) -> Option<T> {
    match (observed, supplied) {
        (Some(observed), Some(supplied)) if observed != supplied => {
            contradiction(issues, &format!("{location}.{field}"));
            None
        }
        (Some(value), _) => {
            ledger.report(
                format!("{location}.{field}"),
                serde_json::to_value(value).unwrap_or(Value::Null),
            );
            Some(value)
        }
        (None, Some(value)) => {
            ledger.supplied(
                format!("{location}.{field}"),
                serde_json::to_value(value).unwrap_or(Value::Null),
            );
            Some(value)
        }
        (None, None) => None,
    }
}

fn validate_reported_stats(
    participant: &Participant,
    entities: &Composition,
    technology: &Technology,
    lifeform: &LifeformBonuses,
    issues: &mut Vec<FieldIssue>,
) {
    let Some(stats) = participant
        .reported_unit_stats
        .as_ref()
        .and_then(Value::as_object)
    else {
        if let Some(boosters) = participant.reported_base_stats_booster.as_ref() {
            // Keeping this object in the candidate is useful evidence, but it
            // never supplies simulator percentages by inference.
            let _ = boosters;
        }
        return;
    };
    let database = entity_stats();
    for &entity in entities.keys() {
        let Some(reported) = stats.get(&entity.to_string()).and_then(Value::as_object) else {
            continue;
        };
        let Some(base) = database.get(&entity) else {
            continue;
        };
        let actual = modified_stats(base, technology, lifeform.get(entity));
        for (field, expected) in [
            ("weapon", actual.weapon),
            ("shield", actual.shield),
            ("armor", actual.armour),
        ] {
            let Some(value) = reported.get(field).and_then(Value::as_f64) else {
                continue;
            };
            if (value - expected).abs() > f64::EPSILON {
                issue(
                    issues,
                    FieldIssueKind::ReportStatMismatch,
                    format!(
                        "{}.reported_unit_stats.{}.{}",
                        participant.slot, entity, field
                    ),
                    format!(
                        "reported {value} does not match reconstructed {expected} under the completed modifiers"
                    ),
                    "supply the matching technology or corrected reported unit statistic",
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct StartingStats {
    weapon: f64,
    shield: f64,
    armour: f64,
}

fn modified_stats(
    base: &EntityStats,
    technology: &Technology,
    lifeform: LifeformBonus,
) -> StartingStats {
    let weapon_modifier =
        1.0 + f64::from(technology.weapon) * 0.1 + f64::from(lifeform.weapon) / 100.0;
    let shield_modifier =
        1.0 + f64::from(technology.shield) * 0.1 + f64::from(lifeform.shield) / 100.0;
    let armour_modifier =
        1.0 + f64::from(technology.armour) * 0.1 + f64::from(lifeform.armour) / 100.0;
    StartingStats {
        weapon: (f64::from(base.weapon) * weapon_modifier).floor(),
        shield: (f64::from(base.shield) * shield_modifier).floor(),
        armour: (f64::from(base.armour) * armour_modifier).floor(),
    }
}

fn validate_universe(
    candidate: &Candidate,
    universe: &PinnedUniverse,
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) {
    if candidate.provenance.community != universe.community
        || candidate.provenance.universe != universe.universe
    {
        issue(
            issues,
            FieldIssueKind::WrongUniverse,
            "universe.identity",
            "pinned universe identity does not match the report provenance",
            "supply public metadata for the report's community and universe",
        );
    }
    if universe.settings.resolve().is_err() {
        issue(
            issues,
            FieldIssueKind::IncompleteUniverse,
            "universe.settings",
            "all universe settings must be explicitly supplied and valid",
            "supply every universe setting, including debris percentages and toggles",
        );
    }
    if universe.source_timestamp.is_none() || universe.source_version.is_none() {
        issue(
            issues,
            FieldIssueKind::IncompleteUniverse,
            "universe.provenance",
            "universe metadata needs a source timestamp and version distinct from battle time",
            "supply the public snapshot timestamp and game version",
        );
    }
    if universe.current && !universe.acknowledged_current {
        issue(
            issues,
            FieldIssueKind::CurrentSnapshotUnacknowledged,
            "universe.acknowledged_current",
            "a current universe snapshot cannot stand in for historical settings without explicit acknowledgement",
            "acknowledge the snapshot as current or supply historical settings",
        );
    }
    ledger.public_metadata(
        "universe.identity",
        serde_json::json!({"community": universe.community, "universe": universe.universe}),
    );
}

fn player_class(value: u8) -> PlayerClass {
    match value {
        1 => PlayerClass::Collector,
        2 => PlayerClass::General,
        3 => PlayerClass::Discoverer,
        _ => PlayerClass::None,
    }
}

fn alliance_class(value: u8) -> AllianceClass {
    match value {
        1 => AllianceClass::Trader,
        2 => AllianceClass::Warrior,
        3 => AllianceClass::Researcher,
        _ => AllianceClass::None,
    }
}

fn json_composition(value: &Composition) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn review_kind(review_required: &[String], location: &str, field: &str) -> FieldIssueKind {
    let prefix = format!("{location}.{field}");
    if review_required
        .iter()
        .any(|entry| entry.starts_with(&prefix) && entry.contains("not revealed"))
    {
        FieldIssueKind::Hidden
    } else if review_required
        .iter()
        .any(|entry| entry.starts_with(&prefix) && entry.contains("missing"))
    {
        FieldIssueKind::Missing
    } else {
        FieldIssueKind::Unknown
    }
}

fn contradiction(issues: &mut Vec<FieldIssue>, location: &str) {
    issue(
        issues,
        FieldIssueKind::Contradictory,
        location,
        "supplied evidence conflicts with the observed report value",
        "correct the supplied value or provide a matching report",
    );
}

fn issue(
    issues: &mut Vec<FieldIssue>,
    kind: FieldIssueKind,
    location: impl Into<String>,
    explanation: impl Into<String>,
    request: impl Into<String>,
) {
    issues.push(FieldIssue {
        kind,
        location: location.into(),
        explanation: explanation.into(),
        evidence_requests: vec![request.into()],
    });
}
