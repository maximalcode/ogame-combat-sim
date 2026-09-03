use crate::cli::ReportArgs;
use combat_ogame_api::reports::{ReportClient, ReportId};
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
