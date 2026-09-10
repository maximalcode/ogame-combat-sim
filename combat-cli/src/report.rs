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
    let path = args.file.as_ref().ok_or_else(|| {
        "report complete requires --file PATH containing a completion artifact".to_owned()
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
    let result = complete_candidate(&input);
    let machine = serde_json::to_string_pretty(&result)
        .map_err(|_| "could not serialize completion result".to_owned())?;
    let mut output = String::new();
    match &result {
        CompletionResult::Verified { input } => {
            output.push_str("Verified combat report candidate\n");
            let _ = write!(
                output,
                "  attacker entities: {}\n  defender entities: {}\n  evidence fields: {}\n",
                input.request.attacker.entities.len(),
                input.request.defender.entities.len(),
                input.evidence.fields.len()
            );
        }
        CompletionResult::Incomplete { issues } => {
            let _ = writeln!(
                output,
                "Incomplete combat report candidate ({} issues)\n",
                issues.len()
            );
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
