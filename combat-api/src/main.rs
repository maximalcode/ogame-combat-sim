//! HTTP server exposing the combat engine.
//!
//! Stateless by design: there is no database, no session and nothing to
//! configure beyond a port. A simulation is a pure function of its request, so
//! the server needs to remember nothing between calls.

use axum::{
    Json, Router,
    http::{Method, StatusCode, header},
    routing::{get, post},
};
use combat_core::{ReportBuilder, Simulator};
use combat_types::{CombatReport, CombatRequest, CombatResults};
use serde::Serialize;
use std::env;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Upper bound on simulations per request, so one caller cannot tie up the
/// server indefinitely. Self-hosters can raise it; the library itself has no
/// limit, and neither does the CLI.
const DEFAULT_MAX_SIMULATIONS: u32 = 1000;

#[derive(Serialize)]
struct SimulationResponse {
    results: CombatResults,
    report: CombatReport,
}

fn max_simulations() -> u32 {
    env::var("MAX_SIMULATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MAX_SIMULATIONS)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "combat_api=info".into()),
        )
        .init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(health))
        .route("/api/simulate", post(simulate))
        .layer(cors);

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    info!("listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    concat!("combat-api ", env!("CARGO_PKG_VERSION"), " — ready")
}

async fn simulate(
    Json(mut request): Json<CombatRequest>,
) -> Result<Json<SimulationResponse>, (StatusCode, String)> {
    let attackers: u32 = request.attacker.entities.values().sum();
    let defenders: u32 = request.defender.entities.values().sum();
    if attackers == 0 && defenders == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "both fleets are empty; there is nothing to simulate".to_owned(),
        ));
    }
    if request.simulations == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "simulations must be at least 1".to_owned(),
        ));
    }

    let cap = max_simulations();
    if request.simulations > cap {
        request.simulations = cap;
    }
    // Downscaling stays on auto: a request for a hundred million ships at full
    // resolution is how you turn a shared server into a heater.
    request.enable_downscaling = None;

    info!(
        "simulating {attackers} attackers vs {defenders} defenders, {} runs",
        request.simulations
    );

    let results = Simulator::new().simulate_multiple(&request);
    let report = ReportBuilder::new().build_summary_report(&request, &results);
    info!(
        "done: {} results in {}ms",
        results.simulations, results.duration_ms
    );

    Ok(Json(SimulationResponse { results, report }))
}
