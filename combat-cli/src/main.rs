//! Command line interface to the combat engine.
//!
//! ```text
//! combat-cli sim -a "cruiser:100,lf:50" -d "lf:1000" --tech 10 -n 1000
//! combat-cli sim --file battle.json
//! combat-cli entities
//! ```
//!
//! The engine is a library and a stateless HTTP server; this is the third way
//! in, and the only one that does not need a port. It applies none of the
//! server's protective limits — see [`cli`] for why.

mod args;
mod cli;
mod fixture;
mod render;

use std::process::ExitCode;

use clap::Parser;
use combat_core::{ReportBuilder, Simulator};

use cli::{Cli, Command, FixtureCommand, SimArgs};

fn main() -> ExitCode {
    // `Cli::parse` exits on its own for `--help` and for malformed argv; what
    // reaches `run` is a syntactically valid command that may still be
    // semantically wrong (an unknown ship, an empty battle).
    let cli = Cli::parse();

    match run(cli.command) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<String, String> {
    match command {
        Command::Sim(args) => simulate(&args),
        Command::Entities => Ok(render::render_entities()),
        Command::Fixture { action } => match action {
            FixtureCommand::Template => Ok(fixture::template()),
            FixtureCommand::Check { paths } => fixture::check(&paths),
            FixtureCommand::Run { paths } => fixture::run(&paths),
        },
    }
}

fn simulate(args: &SimArgs) -> Result<String, String> {
    let request = match &args.file {
        Some(path) => {
            let json = std::fs::read_to_string(path)
                .map_err(|e| format!("could not read {}: {e}", path.display()))?;
            cli::parse_request_json(&json)?
        }
        None => cli::build_request(args)?,
    };

    cli::validate(&request)?;

    let results = Simulator::new().simulate_multiple(&request);
    let report = ReportBuilder::new().build_summary_report(&request, &results);

    let mut output = render::render_report(&request, &results, &report);

    if args.rounds {
        // The first simulation, not a hand-picked one. Any single battle is as
        // representative as any other, and choosing the one that best matches
        // the average would be presenting a curated battle as a typical one.
        let first = results
            .results
            .first()
            .ok_or_else(|| "no simulations were run, so there are no rounds to show".to_owned())?;
        output.push_str(&render::render_rounds(first, results.simulations));
    }

    Ok(output)
}
