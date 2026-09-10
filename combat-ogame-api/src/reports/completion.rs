//! Offline completion of one sanitized combat report.
//!
//! This module is deliberately downstream of parsing.  A parsed candidate is
//! evidence, not a request; callers must explicitly provide the facts whose
//! basis the provider did not establish and pin the universe snapshot before
//! a [`CombatRequest`] is produced.

use super::ReportKind;
use super::model::{Candidate, Composition, Participant};
use combat_core::ModifiedStats;
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

    fn record(&mut self, source: EvidenceSource, location: impl Into<String>, value: Value) {
        self.fields
            .insert(location.into(), EvidenceRecord { source, value });
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
    /// Percentages are simulator units: `50.0` means +50%. `None` means the
    /// report did not establish the lifeform state; `Some(empty)` is an
    /// explicit confirmation that no lifeform modifiers apply.
    #[serde(default)]
    pub lifeform: Option<BTreeMap<u16, PartialLifeformBonus>>,
}

/// A lifeform bonus at the completion boundary. Combat percentages are
/// required for each named entity, but remain optional here so omitted and
/// `null` JSON values cannot become confirmed zeroes during deserialization.
/// Cargo and speed are retained when supplied but are not required to build a
/// combat request.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
pub struct PartialLifeformBonus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shield: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armour: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

/// Evidence used to complete the candidate.  The map is keyed by the stable
/// candidate slots (`A1`, `D1`), so no provider owner id is needed.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct CompletionEvidence {
    #[serde(default)]
    pub participants: BTreeMap<String, ParticipantEvidence>,
    /// Explicit battle-time rapid-fire evidence. This is kept separate from
    /// the pinned universe snapshot because a current snapshot is not
    /// historical proof for an older report.
    #[serde(default)]
    pub historical_rapid_fire: Option<bool>,
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
    pub rapid_fire: Option<bool>,
    pub debris_fleet: Option<u8>,
    pub debris_defence: Option<u8>,
    pub debris_deuterium: Option<bool>,
    pub deuterium_save_factor: Option<u8>,
}

impl PinnedUniverseSettings {
    fn resolve(&self) -> Result<(UniverseSettings, Option<bool>), Vec<&'static str>> {
        let mut missing = Vec::new();
        for (name, present) in [
            ("galaxies", self.galaxies.is_some()),
            ("systems", self.systems.is_some()),
            ("donut_galaxy", self.donut_galaxy.is_some()),
            ("donut_systems", self.donut_systems.is_some()),
            ("fleet_speed", self.fleet_speed.is_some()),
            (
                "deuterium_save_factor",
                self.deuterium_save_factor.is_some(),
            ),
        ] {
            if !present {
                missing.push(name);
            }
        }
        let mut invalid = Vec::new();
        if self.galaxies.is_some_and(|value| !(1..=9).contains(&value)) {
            invalid.push("galaxies");
        }
        if self
            .systems
            .is_some_and(|value| !(1..=499).contains(&value))
        {
            invalid.push("systems");
        }
        if self.fleet_speed.is_some_and(|value| value == 0) {
            invalid.push("fleet_speed");
        }
        if self.debris_fleet.is_some_and(|value| value > 100) {
            invalid.push("debris_fleet");
        }
        if self.debris_defence.is_some_and(|value| value > 100) {
            invalid.push("debris_defence");
        }
        if self.deuterium_save_factor.is_some_and(|value| value > 100) {
            invalid.push("deuterium_save_factor");
        }
        if !missing.is_empty() || !invalid.is_empty() {
            missing.extend(invalid);
            return Err(missing);
        }
        let settings = UniverseSettings {
            galaxies: self.galaxies.expect("checked above"),
            systems: self.systems.expect("checked above"),
            donut_galaxy: self.donut_galaxy.expect("checked above"),
            donut_systems: self.donut_systems.expect("checked above"),
            fleet_speed: self.fleet_speed.expect("checked above"),
            // Debris rates affect derived output, not combat rounds. Unknown
            // rates use the engine's required scalar representation only so a
            // battle can execute; the missing facts remain visible in the
            // assessment limitations below.
            debris_fleet: self
                .debris_fleet
                .unwrap_or_else(|| UniverseSettings::default().debris_fleet),
            debris_defence: self
                .debris_defence
                .unwrap_or_else(|| UniverseSettings::default().debris_defence),
            // Deuterium debris changes derived output only. Keep execution
            // complete with the engine's neutral value and retain the missing
            // fact as an assessment limitation below.
            debris_deuterium: self.debris_deuterium.unwrap_or(false),
            deuterium_save_factor: self.deuterium_save_factor.expect("checked above"),
        };
        Ok((settings, self.rapid_fire))
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
    /// Whether these settings describe the current snapshot. `None` means the
    /// artifact did not make the historical/current choice.
    #[serde(default)]
    pub current: Option<bool>,
    #[serde(default)]
    pub acknowledged_current: Option<bool>,
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
    /// Output metrics whose historical basis is unavailable even though the
    /// request can still execute with the acknowledged current snapshot.
    #[serde(default)]
    pub assessment_limitations: Vec<AssessmentLimitation>,
}

/// A structured downstream assessment limitation. Completion does not run a
/// comparison engine, but it preserves whether an unresolved historical value
/// blocks execution or only one output metric.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct AssessmentLimitation {
    pub metric: String,
    pub location: String,
    pub explanation: String,
    pub affects_execution: bool,
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
// This workflow deliberately keeps validation, evidence reconciliation, and
// request construction together so a verified request cannot bypass one of
// the fail-closed completion checks.
#[allow(clippy::too_many_lines)]
pub fn complete_candidate(input: &CompletionInput) -> CompletionResult {
    let candidate = &input.candidate;
    let mut issues = Vec::new();
    let mut evidence = EvidenceLedger::default();

    evidence.report(
        "battle.provenance",
        serde_json::to_value(&candidate.provenance).unwrap_or(Value::Null),
    );
    if let Some(loot_percentage) = candidate.loot_percentage {
        evidence.report("loot_percentage", Value::from(loot_percentage));
    }

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
    let (Some(attacker), Some(defender), Ok((settings, pinned_rapid_fire))) =
        (attacker, defender, input.universe.settings.resolve())
    else {
        return CompletionResult::Incomplete { issues };
    };
    let rapid_fire = resolve_rapid_fire(
        &input.universe,
        &input.evidence,
        &attacker.party,
        &defender.party,
        pinned_rapid_fire,
        &mut issues,
        &mut evidence,
    );
    if !issues.is_empty() {
        return CompletionResult::Incomplete { issues };
    }

    let request = CombatRequest {
        attacker: attacker.party,
        defender: defender.party,
        universe_settings: Some(settings),
        use_rapid_fire: rapid_fire,
        simulations: 1,
        planet_resources: None,
        plunder_percentage: candidate.loot_percentage.unwrap_or(50),
        // Per-participant technology is already effective.  Keeping these
        // blocks empty is what prevents class bonuses being applied twice.
        attacker_bonuses: None,
        defender_bonuses: None,
        ..CombatRequest::default()
    };
    evidence.record(
        input.universe.source,
        "universe",
        serde_json::to_value(&input.universe).unwrap_or(Value::Null),
    );
    let assessment_limitations = assessment_limitations(&input.universe, &request);
    CompletionResult::Verified {
        input: Box::new(VerifiedBattleInput {
            request,
            evidence,
            observed: candidate.observed.clone(),
            assessment_limitations,
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

// This resolver is intentionally one vertical slice: each accepted value is
// reconciled with report evidence, completion evidence, and the ledger before
// the party can be constructed. Splitting it would obscure that fail-closed
// boundary and make provenance omissions easier to introduce.
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
    if let Some(composition) = participant.entities.as_ref() {
        validate_composition(composition, &location, issues);
    }
    if let Some(composition) = supplied.and_then(|e| e.entities.as_ref()) {
        validate_composition(composition, &location, issues);
    }
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
        participant
            .character_class_id
            .and_then(|value| observed_player_class(value, &location, issues)),
        supplied.and_then(|e| e.player_class),
        &location,
        "player_class",
        issues,
        ledger,
    );
    let alliance_class = resolve_class(
        participant
            .alliance_class_id
            .and_then(|value| observed_alliance_class(value, &location, issues)),
        supplied.and_then(|e| e.alliance_class),
        &location,
        "alliance_class",
        issues,
        ledger,
    );
    let lifeform_evidence = supplied.and_then(|e| e.lifeform.as_ref());
    if lifeform_evidence.is_none() {
        issue(
            issues,
            review_kind(review_required, &location, "lifeform"),
            format!("{location}.lifeform"),
            "reported starting statistics do not establish whether lifeform modifiers were active",
            "supply explicit per-entity lifeform percentages or confirm that no lifeform modifiers apply",
        );
    }
    let technology = resolve_technology(
        participant,
        supplied,
        player_class,
        alliance_class,
        review_required,
        issues,
        ledger,
    );
    let lifeform = lifeform_evidence.map_or_else(LifeformBonuses::default, |evidence| {
        ledger.supplied(
            format!("{location}.lifeform"),
            serde_json::to_value(evidence).unwrap_or(Value::Null),
        );
        let mut resolved = BTreeMap::new();
        for (&entity, bonus) in evidence {
            if !entity_stats().contains_key(&entity) {
                issue(
                    issues,
                    FieldIssueKind::Unsupported,
                    format!("{location}.lifeform.{entity}"),
                    "the supplied lifeform bonus names an entity this simulator does not support",
                    "remove it or supply a bonus for a supported entity",
                );
            }
            let bonus_location = format!("{location}.lifeform.{entity}");
            if let Some(complete) = complete_lifeform_bonus(*bonus, &bonus_location, issues) {
                resolved.insert(entity, complete);
            }
            ledger.supplied(
                bonus_location,
                serde_json::to_value(bonus).unwrap_or(Value::Null),
            );
        }
        resolved.into_iter().collect()
    });

    record_report_evidence(participant, &location, ledger);

    if let (Some(composition), Some(technology)) = (
        participant
            .entities
            .as_ref()
            .or_else(|| supplied.and_then(|e| e.entities.as_ref())),
        technology.as_ref(),
    ) {
        validate_starting_stats(
            participant,
            composition,
            technology,
            lifeform_evidence,
            issues,
        );
        validate_reported_stats(
            participant,
            composition,
            technology,
            lifeform_evidence,
            issues,
        );
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

fn record_report_evidence(participant: &Participant, location: &str, ledger: &mut EvidenceLedger) {
    if let Some(stats) = participant.reported_unit_stats.as_ref() {
        ledger.report(format!("{location}.reported_unit_stats"), stats.clone());
    }
    if let Some(boosters) = participant.reported_base_stats_booster.as_ref() {
        ledger.report(
            format!("{location}.reported_base_stats_booster"),
            boosters.clone(),
        );
    }
}

fn complete_lifeform_bonus(
    bonus: PartialLifeformBonus,
    location: &str,
    issues: &mut Vec<FieldIssue>,
) -> Option<LifeformBonus> {
    let mut complete = true;
    for (field, value) in [
        ("weapon", bonus.weapon),
        ("shield", bonus.shield),
        ("armour", bonus.armour),
    ] {
        match value {
            None => {
                complete = false;
                issue(
                    issues,
                    FieldIssueKind::Missing,
                    format!("{location}.{field}"),
                    format!("lifeform {field} percentage is required for a named entity"),
                    format!("supply the lifeform {field} percentage for this entity"),
                );
            }
            Some(value) if !valid_lifeform_percentage(value) => {
                complete = false;
                issue(
                    issues,
                    FieldIssueKind::Unsupported,
                    format!("{location}.{field}"),
                    format!(
                        "lifeform {field} percentage must be finite and non-negative, got {value}"
                    ),
                    format!("supply a finite non-negative lifeform {field} percentage"),
                );
            }
            Some(_) => {}
        }
    }
    for (field, value) in [("cargo", bonus.cargo), ("speed", bonus.speed)] {
        if let Some(value) = value.filter(|value| !valid_lifeform_percentage(*value)) {
            complete = false;
            issue(
                issues,
                FieldIssueKind::Unsupported,
                format!("{location}.{field}"),
                format!("lifeform {field} percentage must be finite and non-negative, got {value}"),
                format!("supply a finite non-negative lifeform {field} percentage"),
            );
        }
    }
    if !complete {
        return None;
    }
    Some(LifeformBonus {
        weapon: bonus.weapon.expect("checked above"),
        shield: bonus.shield.expect("checked above"),
        armour: bonus.armour.expect("checked above"),
        cargo: bonus.cargo.unwrap_or_default(),
        speed: bonus.speed.unwrap_or_default(),
    })
}

fn validate_starting_stats(
    participant: &Participant,
    composition: &Composition,
    technology: &Technology,
    lifeform_evidence: Option<&BTreeMap<u16, PartialLifeformBonus>>,
    issues: &mut Vec<FieldIssue>,
) {
    let database = entity_stats();
    for &entity in composition.keys() {
        let Some(base) = database.get(&entity) else {
            continue;
        };
        for field in ["weapon", "shield", "armour"] {
            let Some(bonus) = known_lifeform_percentage(lifeform_evidence, entity, field) else {
                continue;
            };
            let valid = match field {
                "weapon" => {
                    let raw_weapon = f64::from(base.weapon)
                        * (1.0 + (f64::from(technology.weapon) * 0.1) + (f64::from(bonus) / 100.0));
                    raw_weapon.is_finite() && (0.0..=f64::from(u32::MAX)).contains(&raw_weapon)
                }
                "shield" | "armour" => {
                    let modified = modified_stat(base, technology, field, bonus);
                    modified.is_finite() && modified >= 0.0
                }
                _ => unreachable!("only combat stats are validated"),
            };
            if !valid {
                issue(
                    issues,
                    FieldIssueKind::Unsupported,
                    format!("{}.lifeform.{}.{}", participant.slot, entity, field),
                    format!(
                        "the supplied lifeform percentage produces an invalid starting {field} statistic"
                    ),
                    "supply a finite percentage that keeps the starting combat statistic representable",
                );
            }
        }
    }
}

fn valid_lifeform_percentage(value: f32) -> bool {
    value.is_finite() && value >= 0.0
}

// Keep the evidence reconciliation together so basis checks and the
// fail-closed class requirements cannot drift apart across helper calls.
#[allow(clippy::too_many_lines)]
fn resolve_technology(
    participant: &Participant,
    supplied: Option<&ParticipantEvidence>,
    player_class: Option<PlayerClass>,
    alliance_class: Option<AllianceClass>,
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
    let effective = match explicit.basis {
        TechnologyBasis::Researched => {
            let (Some(player_class), Some(alliance_class)) = (player_class, alliance_class) else {
                if player_class.is_none() {
                    issue(
                        issues,
                        review_kind(review_required, &participant.slot, "player_class"),
                        format!("{}.player_class", participant.slot),
                        "researched technology cannot be resolved without a confirmed player class",
                        "supply the player class or explicitly confirm no player class",
                    );
                }
                if alliance_class.is_none() {
                    issue(
                        issues,
                        review_kind(review_required, &participant.slot, "alliance_class"),
                        format!("{}.alliance_class", participant.slot),
                        "researched technology cannot be resolved without a confirmed alliance class",
                        "supply the alliance class or explicitly confirm no alliance class",
                    );
                }
                return None;
            };
            let bonuses = PlayerBonuses {
                player_class,
                alliance_class,
                ..PlayerBonuses::default()
            };
            values.effective_levels(Some(&bonuses))
        }
        TechnologyBasis::AlreadyEffective => values,
    };
    // The proxy's percentage-divided-by-ten basis is provisional. When the
    // candidate does name a basis, convert both representations to the same
    // effective levels before comparing them. This keeps a known researched
    // value from bypassing a conflict merely because supplied evidence used
    // the already-effective representation (or vice versa).
    let candidate_basis = match candidate_technology.basis.as_str() {
        "researched" => Some(TechnologyBasis::Researched),
        "already_effective" => Some(TechnologyBasis::AlreadyEffective),
        "reported_combat_bonus_divided_by_ten" | "" => None,
        _ => {
            issue(
                issues,
                FieldIssueKind::UnsupportedBasis,
                format!("{}.basis", participant.slot),
                "the report technology basis is not documented for completion",
                "supply technology evidence with a researched or already-effective basis",
            );
            None
        }
    };
    if let Some(candidate_basis) = candidate_basis {
        let candidate_values = Technology {
            weapon: candidate_technology.weapon.unwrap_or_default(),
            shield: candidate_technology.shield.unwrap_or_default(),
            armour: candidate_technology.armour.unwrap_or_default(),
            ..Technology::default()
        };
        let candidate_effective = match candidate_basis {
            TechnologyBasis::AlreadyEffective => candidate_values,
            TechnologyBasis::Researched => {
                let (Some(player_class), Some(alliance_class)) = (player_class, alliance_class)
                else {
                    if player_class.is_none() {
                        issue(
                            issues,
                            review_kind(review_required, &participant.slot, "player_class"),
                            format!("{}.player_class", participant.slot),
                            "known researched report technology cannot be reconciled without a confirmed player class",
                            "supply the player class or explicitly confirm no player class",
                        );
                    }
                    if alliance_class.is_none() {
                        issue(
                            issues,
                            review_kind(review_required, &participant.slot, "alliance_class"),
                            format!("{}.alliance_class", participant.slot),
                            "known researched report technology cannot be reconciled without a confirmed alliance class",
                            "supply the alliance class or explicitly confirm no alliance class",
                        );
                    }
                    return None;
                };
                let bonuses = PlayerBonuses {
                    player_class,
                    alliance_class,
                    ..PlayerBonuses::default()
                };
                candidate_values.effective_levels(Some(&bonuses))
            }
        };
        for (field, reported, candidate_completed, supplied_completed) in [
            (
                "weapon",
                candidate_technology.weapon,
                candidate_effective.weapon,
                effective.weapon,
            ),
            (
                "shield",
                candidate_technology.shield,
                candidate_effective.shield,
                effective.shield,
            ),
            (
                "armour",
                candidate_technology.armour,
                candidate_effective.armour,
                effective.armour,
            ),
        ] {
            if reported.is_some() && candidate_completed != supplied_completed {
                contradiction(issues, &format!("{}.{}", participant.slot, field));
            }
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
    lifeform_evidence: Option<&BTreeMap<u16, PartialLifeformBonus>>,
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
        for (field, stat) in [
            ("weapon", "weapon"),
            ("shield", "shield"),
            ("armor", "armour"),
        ] {
            let Some(value) = reported.get(field).and_then(Value::as_f64) else {
                continue;
            };
            let Some(bonus) = known_lifeform_percentage(lifeform_evidence, entity, stat) else {
                continue;
            };
            let expected = modified_stat(base, technology, stat, bonus);
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

/// Return a lifeform modifier only when the evidence establishes the value for
/// this entity and stat. An explicitly supplied map establishes zero for an
/// entity it does not name; an omitted map establishes nothing. Partial named
/// entries remain unknown only for their omitted combat stats.
fn known_lifeform_percentage(
    lifeform_evidence: Option<&BTreeMap<u16, PartialLifeformBonus>>,
    entity: u16,
    field: &str,
) -> Option<f32> {
    let evidence = lifeform_evidence?;
    let Some(bonus) = evidence.get(&entity) else {
        return Some(0.0);
    };
    let value = match field {
        "weapon" => bonus.weapon,
        "shield" => bonus.shield,
        "armour" => bonus.armour,
        _ => unreachable!("only combat stats have lifeform modifiers"),
    }?;
    valid_lifeform_percentage(value).then_some(value)
}

fn validate_composition(composition: &Composition, location: &str, issues: &mut Vec<FieldIssue>) {
    if composition.is_empty() {
        issue(
            issues,
            FieldIssueKind::Missing,
            format!("{location}.entities"),
            "the participant composition is empty, so there is no runnable battle fleet",
            "supply at least one supported ship or defence with its count",
        );
    }
    let database = entity_stats();
    for (&entity, &count) in composition {
        if !database.contains_key(&entity) {
            issue(
                issues,
                FieldIssueKind::Unsupported,
                format!("{location}.entities.{entity}"),
                "the participant composition contains an entity this simulator does not support",
                "replace it with an entity listed by the simulator",
            );
        }
        if count == 0 {
            issue(
                issues,
                FieldIssueKind::Missing,
                format!("{location}.entities.{entity}"),
                "the participant composition must contain a positive count",
                "supply a positive count for this entity or remove it",
            );
        }
    }
}

fn modified_stat(
    base: &EntityStats,
    technology: &Technology,
    field: &str,
    lifeform_percentage: f32,
) -> f64 {
    let lifeform = match field {
        "weapon" => LifeformBonus {
            weapon: lifeform_percentage,
            ..LifeformBonus::default()
        },
        "shield" => LifeformBonus {
            shield: lifeform_percentage,
            ..LifeformBonus::default()
        },
        "armour" => LifeformBonus {
            armour: lifeform_percentage,
            ..LifeformBonus::default()
        },
        _ => unreachable!("only combat stats are reconstructed"),
    };
    let modified = ModifiedStats::calculate(base, technology, lifeform);
    match field {
        "weapon" => f64::from(modified.weapon),
        "shield" => f64::from(modified.shield),
        // Report `armor` is the combat hull value. The engine converts the
        // armour resource stat into hull points before flooring it.
        "armour" => f64::from(modified.hull),
        _ => unreachable!("only combat stats are reconstructed"),
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
    if let Err(fields) = universe.settings.resolve() {
        for field in fields {
            let missing = match field {
                "galaxies" => universe.settings.galaxies.is_none(),
                "systems" => universe.settings.systems.is_none(),
                "donut_galaxy" => universe.settings.donut_galaxy.is_none(),
                "donut_systems" => universe.settings.donut_systems.is_none(),
                "fleet_speed" => universe.settings.fleet_speed.is_none(),
                "deuterium_save_factor" => universe.settings.deuterium_save_factor.is_none(),
                _ => false,
            };
            issue(
                issues,
                if missing {
                    FieldIssueKind::Missing
                } else {
                    FieldIssueKind::Unsupported
                },
                format!("universe.settings.{field}"),
                if missing {
                    format!("universe setting {field} is required and has not been supplied")
                } else {
                    format!("universe setting {field} is outside the supported range")
                },
                format!("supply a valid value for universe setting {field}"),
            );
        }
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
    if universe.current.is_none() {
        issue(
            issues,
            FieldIssueKind::Missing,
            "universe.current",
            "the pinned snapshot must explicitly state whether it is current or historical",
            "supply current: true or current: false for the pinned universe settings",
        );
    } else if universe.current == Some(true) && universe.acknowledged_current != Some(true) {
        issue(
            issues,
            FieldIssueKind::CurrentSnapshotUnacknowledged,
            "universe.acknowledged_current",
            "a current universe snapshot cannot stand in for historical settings without explicit acknowledgement",
            "acknowledge the snapshot as current or supply historical settings",
        );
    } else if universe.current == Some(false) && universe.acknowledged_current == Some(true) {
        issue(
            issues,
            FieldIssueKind::Contradictory,
            "universe.acknowledged_current",
            "a historical snapshot cannot also be acknowledged as current",
            "set acknowledged_current to false or mark the snapshot current",
        );
    }
    ledger.record(
        universe.source,
        "universe.identity",
        serde_json::json!({"community": universe.community, "universe": universe.universe}),
    );
    if let Some(timestamp) = universe.source_timestamp {
        ledger.record(
            universe.source,
            "universe.source_timestamp",
            Value::from(timestamp),
        );
    }
    if let Some(version) = universe.source_version.as_ref() {
        ledger.record(
            universe.source,
            "universe.source_version",
            Value::from(version.clone()),
        );
    }
}

// These inputs stay separate because the resolver must compare report facts,
// pinned metadata, and supplied battle-time evidence before returning a
// request value or recording it in the ledger.
#[allow(clippy::too_many_arguments)]
fn resolve_rapid_fire(
    universe: &PinnedUniverse,
    completion_evidence: &CompletionEvidence,
    attacker: &PartyData,
    defender: &PartyData,
    pinned_rapid_fire: Option<bool>,
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) -> bool {
    let supplied = completion_evidence.historical_rapid_fire;
    if let Some(value) = supplied {
        if universe.current == Some(false)
            && universe
                .settings
                .rapid_fire
                .is_some_and(|pinned| pinned != value)
        {
            contradiction(issues, "universe.settings.rapid_fire");
        } else {
            ledger.supplied(
                "universe.settings.rapid_fire.historical",
                Value::from(value),
            );
            return value;
        }
    }

    let rapid_fire_applies = rapid_fire_can_affect(attacker, defender);
    // An acknowledged current snapshot is still only a current snapshot. Its
    // source timestamp does not establish that the setting remained unchanged
    // through the report event, in either timestamp ordering.
    if pinned_rapid_fire.is_none() && rapid_fire_applies {
        issue(
            issues,
            FieldIssueKind::Missing,
            "universe.settings.rapid_fire",
            "the historical rapid-fire setting is required for this battle's execution",
            "supply a pinned or explicit historical rapid-fire value for the report event",
        );
    } else if acknowledged_current_snapshot_is_not_historical(universe) && rapid_fire_applies {
        issue(
            issues,
            FieldIssueKind::Missing,
            "universe.settings.rapid_fire",
            "the acknowledged current rapid-fire setting is not historical proof for this battle's execution",
            "supply explicit historical rapid-fire evidence for the report event",
        );
    }
    pinned_rapid_fire.unwrap_or(false)
}

fn rapid_fire_can_affect(attacker: &PartyData, defender: &PartyData) -> bool {
    rapid_fire_from_side_can_affect(&attacker.entities, &defender.entities)
        || rapid_fire_from_side_can_affect(&defender.entities, &attacker.entities)
}

fn rapid_fire_from_side_can_affect(
    attackers: &combat_types::FleetComposition,
    defenders: &combat_types::FleetComposition,
) -> bool {
    attackers.iter().any(|(&attacker, &count)| {
        count > 0
            && entity_stats().get(&attacker).is_some_and(|stats| {
                stats
                    .rapid_fire_against
                    .keys()
                    .any(|target| defenders.get(target).is_some_and(|count| *count > 0))
            })
    })
}

fn assessment_limitations(
    universe: &PinnedUniverse,
    request: &CombatRequest,
) -> Vec<AssessmentLimitation> {
    let mut limitations = Vec::new();
    for field in ["debris_fleet", "debris_defence", "debris_deuterium"] {
        if !debris_setting_applies(field, request, &universe.settings, false) {
            continue;
        }
        let missing = match field {
            "debris_fleet" => universe.settings.debris_fleet.is_none(),
            "debris_defence" => universe.settings.debris_defence.is_none(),
            "debris_deuterium" => universe.settings.debris_deuterium.is_none(),
            _ => false,
        };
        if missing {
            for metric in [
                "generated_debris",
                "moon_chance",
                "recyclers_needed",
                "attacker_profit",
                "defender_profit",
            ] {
                limitations.push(AssessmentLimitation {
                    metric: metric.to_owned(),
                    location: format!("universe.settings.{field}"),
                    explanation: format!(
                        "the battle-time {field} setting is unknown; {metric} cannot be assessed"
                    ),
                    affects_execution: false,
                });
            }
        }
    }
    if !acknowledged_current_snapshot_is_not_historical(universe) {
        return limitations;
    }

    let Some(field) = ["debris_fleet", "debris_defence", "debris_deuterium"]
        .into_iter()
        .find(|field| debris_setting_applies(field, request, &universe.settings, true))
    else {
        return limitations;
    };
    for metric in [
        "generated_debris",
        "moon_chance",
        "recyclers_needed",
        "attacker_profit",
        "defender_profit",
    ] {
        limitations.push(AssessmentLimitation {
            metric: metric.to_owned(),
            location: format!("universe.settings.{field}"),
            explanation: format!(
                "the acknowledged current universe snapshot is not historical proof for the report event; {metric} cannot be assessed against the battle-time debris rules"
            ),
            affects_execution: false,
        });
    }
    limitations
}

fn acknowledged_current_snapshot_is_not_historical(universe: &PinnedUniverse) -> bool {
    universe.current == Some(true) && universe.acknowledged_current == Some(true)
}

/// Return whether an unknown debris setting can affect a modeled output for
/// the completed composition.  The optional rate is deliberately treated as
/// potentially non-zero when it is unknown; only a known zero rate proves that
/// a deuterium switch cannot contribute through that side of the battle.
fn debris_setting_applies(
    field: &str,
    request: &CombatRequest,
    settings: &PinnedUniverseSettings,
    historical_unknown: bool,
) -> bool {
    match field {
        "debris_fleet" => {
            request
                .attacker
                .entities
                .iter()
                .any(|(&entity, &count)| count > 0 && entity < 400)
                || request
                    .defender
                    .entities
                    .iter()
                    .any(|(&entity, &count)| count > 0 && entity < 400)
        }
        "debris_defence" => request
            .defender
            .entities
            .iter()
            .any(|(&entity, &count)| count > 0 && (400..500).contains(&entity)),
        "debris_deuterium" => {
            let fleet_rate = (!historical_unknown)
                .then_some(settings.debris_fleet)
                .flatten();
            let defence_rate = (!historical_unknown)
                .then_some(settings.debris_defence)
                .flatten();
            deuterium_cost_can_contribute(&request.attacker.entities, fleet_rate, false)
                || deuterium_cost_can_contribute(&request.defender.entities, fleet_rate, false)
                || deuterium_cost_can_contribute(&request.defender.entities, defence_rate, true)
        }
        _ => false,
    }
}

fn deuterium_cost_can_contribute(
    entities: &combat_types::FleetComposition,
    applicable_rate: Option<u8>,
    defences_only: bool,
) -> bool {
    entities.iter().any(|(&entity, &count)| {
        count > 0
            && if defences_only {
                (400..500).contains(&entity)
            } else {
                entity < 400
            }
            && applicable_rate != Some(0)
            && entity_stats()
                .get(&entity)
                .is_some_and(|stats| stats.cost_deuterium > 0)
    })
}

fn player_class(value: u8) -> PlayerClass {
    match value {
        1 => PlayerClass::Collector,
        2 => PlayerClass::General,
        3 => PlayerClass::Discoverer,
        _ => PlayerClass::None,
    }
}

fn observed_player_class(
    value: u8,
    location: &str,
    issues: &mut Vec<FieldIssue>,
) -> Option<PlayerClass> {
    if value <= 3 {
        Some(player_class(value))
    } else {
        issue(
            issues,
            FieldIssueKind::Unsupported,
            format!("{location}.player_class"),
            "the report contains an unsupported player class identifier",
            "supply a supported player class identifier or explicit class evidence",
        );
        None
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

fn observed_alliance_class(
    value: u8,
    location: &str,
    issues: &mut Vec<FieldIssue>,
) -> Option<AllianceClass> {
    if value <= 3 {
        Some(alliance_class(value))
    } else {
        issue(
            issues,
            FieldIssueKind::Unsupported,
            format!("{location}.alliance_class"),
            "the report contains an unsupported alliance class identifier",
            "supply a supported alliance class identifier or explicit class evidence",
        );
        None
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
