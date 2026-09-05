use crate::cli::ReportArgs;
use combat_ogame_api::reports::{
    CompletionInput, CompletionResult, ReportClient, ReportId, complete_candidate,
};
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
        .map_err(|_| format!("could not read completion artifact {}", path.display()))?;
    let input: CompletionInput = serde_json::from_str(&json)
        .map_err(|error| format!("invalid completion artifact JSON: {error}"))?;
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
