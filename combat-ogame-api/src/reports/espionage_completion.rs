//! Adapt a revealed snapshot to the shared participant completion rules.
use super::{
    CompletionInput, Composition, EvidenceLedger, FieldIssue, FieldIssueKind, Participant,
    ParticipantEvidence, ReportKind, Value, issue, json_composition,
};

pub(super) fn prepare(
    input: &CompletionInput,
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) -> Option<CompletionInput> {
    if input.candidate.report_kind != ReportKind::Espionage {
        return None;
    }
    let mut prepared = input.clone();
    prepared.candidate.attackers = vec![Participant {
        slot: "A1".into(),
        ..Default::default()
    }];
    prepared.candidate.observed = None;
    for participant in &mut prepared.candidate.defenders {
        let supplied = prepared
            .evidence
            .participants
            .entry(participant.slot.clone())
            .or_default();
        participant.entities = composition(participant, supplied, issues, ledger);
        // Espionage combat-information has no established per-unit combat-report units.
        participant.reported_unit_stats = None;
        let visibility = participant.espionage_visibility.clone().unwrap_or_default();
        ledger.report(
            format!("{}.espionage_visibility", participant.slot),
            serde_json::to_value(&visibility).unwrap_or(Value::Null),
        );
        if visibility.failed_research != Some(false) {
            participant.technology = super::super::TechnologyCandidate::default();
        }
        if supplied.technology.is_none() {
            for (field, value) in [
                ("weapon", participant.technology.weapon),
                ("shield", participant.technology.shield),
                ("armour", participant.technology.armour),
            ] {
                if value.is_none() {
                    issue(
                        issues,
                        visibility_issue(visibility.failed_research),
                        format!("{}.technology.{field}", participant.slot),
                        "this research level was not established by the snapshot",
                        "supply the missing research level and its basis",
                    );
                }
            }
        }
    }
    Some(prepared)
}

fn composition(
    participant: &Participant,
    supplied: &ParticipantEvidence,
    issues: &mut Vec<FieldIssue>,
    ledger: &mut EvidenceLedger,
) -> Option<Composition> {
    let mut combined = Composition::new();
    let mut complete = true;
    let visibility = participant.espionage_visibility.clone().unwrap_or_default();
    for (field, observed, defence, flag) in [
        ("ships", &participant.ships, false, visibility.failed_ships),
        (
            "defenses",
            &participant.defenses,
            true,
            visibility.failed_defense,
        ),
    ] {
        let observed = observed.as_ref().filter(|_| flag == Some(false));
        let location = format!("{}.{}", participant.slot, field);
        let provided = supplied.entities.as_ref().map(|entities| {
            entities
                .iter()
                .filter(|(id, _)| (**id >= 400) == defence)
                .map(|(&id, &count)| (id, count))
                .collect::<Composition>()
        });
        let value = match (observed, provided) {
            (Some(observed), _) => {
                ledger.report(location, json_composition(observed));
                Some(observed.clone())
            }
            (None, Some(provided)) => {
                ledger.supplied(location, json_composition(&provided));
                Some(provided)
            }
            (None, None) => {
                complete = false;
                issue(
                    issues,
                    visibility_issue(flag),
                    location,
                    "the snapshot does not reveal this composition",
                    "supply the complete defender composition, including explicit empty groups",
                );
                None
            }
        };
        if let Some(value) = value {
            combined.extend(value);
        }
    }
    complete.then_some(combined)
}

pub(super) fn record_provenance(input: &CompletionInput, ledger: &mut EvidenceLedger) {
    if input.candidate.report_kind != ReportKind::Espionage {
        return;
    }
    // The combined composition is derived from separately sourced groups.
    for participant in &input.candidate.defenders {
        ledger
            .fields
            .remove(&format!("{}.entities", participant.slot));
    }
}

fn visibility_issue(flag: Option<bool>) -> FieldIssueKind {
    match flag {
        Some(true) => FieldIssueKind::Hidden,
        Some(false) => FieldIssueKind::Missing,
        None => FieldIssueKind::Unknown,
    }
}
