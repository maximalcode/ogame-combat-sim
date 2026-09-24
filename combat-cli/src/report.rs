use crate::cli::ReportArgs;
use combat_ogame_api::reports::{
    CompletionEvidence, CompletionInput, CompletionResult, PinnedUniverse, ReportClient, ReportId,
    complete_candidate, resolve_current_universe,
};
use combat_ogame_api::{OGameClient, Universe};
use serde::Deserialize;
use std::fmt::Write as _;
use std::io::Read;

pub fn import(args: &ReportArgs) -> Result<String, String> {
    if !args.allow_proxy_transfer {
        return Err("report retrieval sends your ID to the third-party proxy https://ogapi.faw-kes.de, which advertises caching; local non-retention does not control proxy retention. Use --allow-proxy-transfer to proceed. Independent processes share its 10 requests per 60 seconds quota".to_owned());
    }
    let input: Box<dyn Read> = match &args.file {
        Some(path) => {
            Box::new(std::fs::File::open(path).map_err(|_| "could not open the report-ID file")?)
        }
        None => Box::new(std::io::stdin()),
    };
    let mut text = String::new();
    input
        .take(257)
        .read_to_string(&mut text)
        .map_err(|_| "could not read the report ID as UTF-8")?;
    if text.len() > 256 {
        return Err("report-ID input exceeds the 256-byte limit".to_owned());
    }
    let id = ReportId::parse(text.trim()).map_err(|error| error.to_string())?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "could not start the report client")?;
    let client = ReportClient::new().map_err(|error| error.to_string())?;
    let candidate = runtime
        .block_on(client.fetch(&id))
        .map_err(|error| error.to_string())?;
    let output = serde_json::to_string_pretty(&candidate)
        .map_err(|_| "could not serialize the sanitized candidate")?;
    Ok(format!("{output}\n"))
}

/// Complete a local structured artifact through the same library result used
/// by future UI clients. The input contains a sanitized candidate, explicit
/// evidence, and a pinned universe; it never accepts a `CombatRequest`.
pub fn complete(args: &ReportArgs) -> Result<String, String> {
    workflow(args, false)
}

pub fn compare(args: &ReportArgs) -> Result<String, String> {
    if args.resolve_current {
        return Err("report compare is offline; pin universe settings in the local completion artifact first".to_owned());
    }
    workflow(args, true)
}

fn workflow(args: &ReportArgs, comparison: bool) -> Result<String, String> {
    let path = args.file.as_ref().ok_or_else(|| {
        "report completion/comparison requires --file PATH containing a completion artifact"
            .to_owned()
    })?;
    let json = std::fs::read_to_string(path)
        .map_err(|_| "could not read completion artifact".to_owned())?;
    let artifact: CompletionArtifact = serde_json::from_str(&json)
        .map_err(|_| {
            "invalid completion artifact JSON; expected a sanitized candidate, evidence and pinned universe"
                .to_owned()
        })?;
    let universe = if args.resolve_current {
        let universe_name = format!(
            "s{}-{}",
            artifact.candidate.provenance.universe, artifact.candidate.provenance.community
        );
        let universe = Universe::new(universe_name).map_err(|error| error.to_string())?;
        let client =
            OGameClient::new(universe, &args.cache_dir).map_err(|error| error.to_string())?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "could not start the public metadata client".to_owned())?;
        let mut pinned = runtime
            .block_on(resolve_current_universe(&artifact.candidate, &client))
            .map_err(|error| error.to_string())?;
        if args.acknowledge_current {
            pinned.acknowledged_current = Some(true);
        }
        pinned
    } else {
        artifact.universe.ok_or_else(|| {
            "completion artifact has no pinned universe; supply one or use --resolve-current"
                .to_owned()
        })?
    };
    let input = CompletionInput {
        candidate: artifact.candidate,
        evidence: artifact.evidence,
        universe,
    };
    let label = match input.candidate.report_kind {
        combat_ogame_api::reports::ReportKind::Combat => "combat report candidate",
        combat_ogame_api::reports::ReportKind::Espionage => "espionage scenario",
    };
    let result = complete_candidate(&input);
    if comparison {
        if let CompletionResult::Verified { input } = &result {
            let result = combat_ogame_api::reports::compare_battle(input).map_err(str::to_owned)?;
            return render_comparison(&result);
        }
    }
    let machine = serde_json::to_string_pretty(&result)
        .map_err(|_| "could not serialize completion result".to_owned())?;
    let mut output = String::new();
    match &result {
        CompletionResult::Verified { input } => {
            let _ = writeln!(output, "Verified {label}");
            if input.observed.is_none() {
                output.push_str("  Snapshot evidence only; no observed battle comparison or claim about another time.\n");
            }
            let _ = write!(
                output,
                "  attacker entities: {}\n  defender entities: {}\n  evidence fields: {}\n",
                input.request.attacker.entities.len(),
                input.request.defender.entities.len(),
                input.evidence.fields.len()
            );
        }
        CompletionResult::Incomplete { issues } => {
            let _ = writeln!(output, "Incomplete {label} ({} issues)\n", issues.len());
            for issue in issues {
                let _ = writeln!(
                    output,
                    "  {} at {}: {}\n    evidence: {}\n",
                    serde_json::to_string(&issue.kind).unwrap_or_else(|_| "unknown".to_owned()),
                    issue.location,
                    issue.explanation,
                    issue.evidence_requests.join("; ")
                );
            }
        }
    }
    output.push_str("\nMachine-readable result:\n");
    output.push_str(&machine);
    output.push('\n');
    Ok(output)
}

/// The CLI artifact keeps the universe optional only to support the explicit
/// `--resolve-current` path. The library boundary remains `CompletionInput`,
/// which always carries a pinned snapshot before simulation can start.
#[derive(Debug, Deserialize)]
struct CompletionArtifact {
    candidate: combat_ogame_api::reports::Candidate,
    #[serde(default)]
    evidence: CompletionEvidence,
    #[serde(default)]
    universe: Option<PinnedUniverse>,
}

fn render_comparison(
    result: &combat_ogame_api::reports::BattleComparison,
) -> Result<String, String> {
    let machine = serde_json::to_string_pretty(result)
        .map_err(|_| "could not serialize comparison result".to_owned())?;
    Ok(format!(
        "{}\nMachine-readable result:\n{machine}\n",
        result.render_text()
    ))
}

/// A separate offline action that cannot confer verified-completion status.
pub fn compare_stats(args: &ReportArgs) -> Result<String, String> {
    use std::io::Read as _;
    if args.resolve_current || args.allow_proxy_transfer {
        return Err("compare-stats is offline; supply all inputs in the local artifact".to_owned());
    }
    let path = args
        .file
        .as_ref()
        .ok_or("compare-stats requires --file PATH")?;
    let file = std::fs::File::open(path).map_err(|_| "could not read reported-stat artifact")?;
    let mut bytes = Vec::new();
    let limit = combat_ogame_api::reports::MAX_REPORT_BYTES;
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "could not read reported-stat artifact")?;
    if bytes.len() > limit {
        return Err("reported-stat artifact exceeds 2 MiB".to_owned());
    }
    let input:combat_ogame_api::reports::ReportedComparisonInput=serde_json::from_slice(&bytes).map_err(|_|"invalid reported-stat artifact; supply explicit count, weapon, shield and hull fields")?;
    let result = combat_ogame_api::reports::compare_reported(&input).map_err(str::to_owned)?;
    let machine = serde_json::to_string_pretty(&result)
        .map_err(|_| "could not serialize reported-stat comparison")?;
    Ok(format!(
        "Conditional reported-stat comparison: {} simulations\n{}\n{}\n\nMachine-readable result:\n{machine}\n",
        result.run_count, result.method, result.limitations
    ))
}

#[cfg(test)]
mod tests {
    use super::render_comparison;
    use combat_ogame_api::reports::{EvidenceLedger, VerifiedBattleInput, compare_with};
    use combat_types::{CombatRequest, SimulationResult};

    #[test]
    fn cli_renders_controlled_suspicion_ceiling_uncertainty_and_redaction() {
        for (wins_per_hundred, status, runs) in [
            (0, "suspicious", 200),
            (5, "statistically_uncertain", 1000),
            (100, "unremarkable", 50),
        ] {
            let input = VerifiedBattleInput {
                request: CombatRequest::default(),
                evidence: EvidenceLedger::default(),
                observed: Some(
                    serde_json::json!({"winner":"attacker", "report_id":"private-report-secret"}),
                ),
                assessment_limitations: vec![],
            };
            let mut index = 0;
            let comparison = compare_with(&input, |request| {
                (0..request.simulations).map(|_| {
                    let outcome = if index % 100 < wins_per_hundred {"AttackersWin"} else {"Draw"};
                    index += 1;
                    serde_json::from_value::<SimulationResult>(serde_json::json!({
                        "outcome":outcome,"rounds":6,"attacker_losses":{},"defender_losses":{},
                        "attacker_remaining":{},"defender_remaining":{},
                        "debris_field":{"metal":0,"crystal":0,"deuterium":0},
                        "loot":{"metal":0,"crystal":0,"deuterium":0},"attacker_profit":0,"defender_profit":0
                    })).unwrap()
                }).collect()
            }).unwrap();
            let text = render_comparison(&comparison).unwrap();
            assert!(text.contains(&format!("outcome: {status}")));
            assert!(text.contains("not_assessable"));
            assert!(!text.contains("private-report-secret"));
            let machine: serde_json::Value =
                serde_json::from_str(text.split("Machine-readable result:\n").nth(1).unwrap())
                    .unwrap();
            assert_eq!(machine["run_count"], runs);
            assert_eq!(machine["metrics"][0]["status"], status);
        }
    }
}
