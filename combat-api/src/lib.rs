//! HTTP application and lifecycle for the combat API.
//!
//! The request handler admits a bounded number of simulations and runs each
//! admitted computation on an owned operating-system thread.  That keeps the
//! CPU-bound engine away from Tokio's request executor and, unlike an aborted
//! async future, keeps the admission permit until the computation really ends.

use axum::{
    Json, Router,
    http::{Method, StatusCode, header},
    routing::{get, post},
};
use combat_core::{ReportBuilder, Simulator};
use combat_types::{CombatReport, CombatRequest, CombatResults};
use serde::Serialize;
use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use tokio::{
    sync::{Semaphore, oneshot, watch},
    time::timeout,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

/// Maximum simulations accepted from one request by default.
pub const DEFAULT_MAX_SIMULATIONS: u32 = 1000;
/// Maximum concurrently admitted simulation computations by default.
pub const DEFAULT_MAX_CONCURRENT_SIMULATIONS: usize = 1;
/// Grace period used after SIGTERM by default.
pub const DEFAULT_SHUTDOWN_GRACE_SECONDS: u64 = 10;

/// The JSON envelope returned by `POST /api/simulate`.
#[derive(Debug, Serialize)]
pub struct SimulationResponse {
    pub results: CombatResults,
    pub report: CombatReport,
}

/// The CPU work performed for an admitted request.
pub trait SimulationRunner: Send + Sync + 'static {
    fn run(&self, request: CombatRequest) -> SimulationResponse;
}

struct EngineRunner;

impl SimulationRunner for EngineRunner {
    fn run(&self, request: CombatRequest) -> SimulationResponse {
        let results = Simulator::new().simulate_multiple(&request);
        let report = ReportBuilder::new().build_summary_report(&request, &results);
        SimulationResponse { results, report }
    }
}

/// Parsed process configuration. Invalid positive-integer lifecycle settings
/// fail before binding a listener, so a typo cannot silently disable the
/// admission or shutdown contract.
#[derive(Debug, Clone, Copy)]
pub struct ServerConfig {
    pub port: u16,
    pub max_simulations: u32,
    pub max_concurrent_simulations: usize,
    pub shutdown_grace: Duration,
}

impl ServerConfig {
    /// Read process configuration from the documented environment variables.
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            port: parse_or_default("PORT", 3000)?,
            max_simulations: parse_or_default("MAX_SIMULATIONS", DEFAULT_MAX_SIMULATIONS)?,
            max_concurrent_simulations: parse_positive_or_default(
                "MAX_CONCURRENT_SIMULATIONS",
                DEFAULT_MAX_CONCURRENT_SIMULATIONS,
            )?,
            shutdown_grace: Duration::from_secs(parse_positive_or_default(
                "SHUTDOWN_GRACE_SECONDS",
                DEFAULT_SHUTDOWN_GRACE_SECONDS,
            )?),
        })
    }
}

fn parse_or_default<T>(name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|error| format!("{name} must be a valid value: {error}")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(format!("could not read {name}: {error}")),
    }
}

fn parse_positive_or_default<T>(name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr + PartialEq + From<u8>,
    T::Err: std::fmt::Display,
{
    let value = parse_or_default(name, default)?;
    if value == T::from(0) {
        Err(format!("{name} must be a positive integer"))
    } else {
        Ok(value)
    }
}

/// Shared admission state. `close` makes the no-new-work transition atomic
/// with respect to permit acquisition; health does not acquire this semaphore.
#[derive(Clone)]
pub struct AppState {
    max_simulations: u32,
    permits: Arc<Semaphore>,
    accepting: Arc<AtomicBool>,
    runner: Arc<dyn SimulationRunner>,
}

impl AppState {
    #[must_use]
    pub fn new(config: ServerConfig) -> Self {
        Self::with_runner(config, Arc::new(EngineRunner))
    }

    #[must_use]
    pub fn with_runner(config: ServerConfig, runner: Arc<dyn SimulationRunner>) -> Self {
        Self {
            max_simulations: config.max_simulations,
            permits: Arc::new(Semaphore::new(config.max_concurrent_simulations)),
            accepting: Arc::new(AtomicBool::new(true)),
            runner,
        }
    }

    /// Stop admitting new simulation work. Existing worker threads retain
    /// their permits until their computation returns.
    pub fn stop_admission(&self) {
        self.accepting.store(false, Ordering::Release);
        self.permits.close();
    }

    #[must_use]
    pub fn is_accepting(&self) -> bool {
        self.accepting.load(Ordering::Acquire)
    }
}

/// Build the HTTP router. This is the public seam used by lifecycle tests and
/// by the binary's listener setup.
pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/", get(health))
        .route("/api/simulate", post(simulate))
        .layer(cors)
        .with_state(state)
}

async fn health() -> &'static str {
    concat!("combat-api ", env!("CARGO_PKG_VERSION"), " — ready")
}

async fn simulate(
    axum::extract::State(state): axum::extract::State<AppState>,
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

    if !state.is_accepting() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "server is shutting down".to_owned(),
        ));
    }
    let permit = state.permits.clone().try_acquire_owned().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "simulation capacity exhausted".to_owned(),
        )
    })?;

    let cap = state.max_simulations.max(1);
    if request.simulations > cap {
        request.simulations = cap;
    }
    request.enable_downscaling = None;
    info!(
        "simulating {attackers} attackers vs {defenders} defenders, {} runs",
        request.simulations
    );

    let runner = state.runner.clone();
    let (sender, receiver) = oneshot::channel();
    thread::Builder::new()
        .name("combat-api-simulation".to_owned())
        .spawn(move || {
            let response = runner.run(request);
            // The permit is deliberately dropped only after run returns. If
            // the HTTP receiver was dropped, this still protects capacity.
            drop(permit);
            let _ = sender.send(response);
        })
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("could not start simulation worker: {error}"),
            )
        })?;

    let response = receiver.await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "simulation worker stopped unexpectedly".to_owned(),
        )
    })?;
    info!(
        "done: {} results in {}ms",
        response.results.simulations, response.results.duration_ms
    );
    Ok(Json(response))
}

async fn wait_for_shutdown(state: AppState, shutdown: watch::Sender<bool>) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result { warn!(%error, "failed to listen for Ctrl-C"); }
            }
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        warn!(%error, "failed to listen for Ctrl-C");
    }
    state.stop_admission();
    let _ = shutdown.send(true);
}

/// Run the API until a signal, allowing admitted requests a finite drain.
pub async fn run(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState::new(config);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    info!("listening on http://0.0.0.0:{}", config.port);

    let (shutdown_sender, shutdown_receiver) = watch::channel(false);
    let shutdown = async move {
        let mut receiver = shutdown_receiver;
        while !*receiver.borrow() && receiver.changed().await.is_ok() {}
    };
    let server = axum::serve(listener, app(state.clone()))
        .with_graceful_shutdown(shutdown)
        .into_future();
    tokio::pin!(server);
    let signal = wait_for_shutdown(state, shutdown_sender);
    tokio::pin!(signal);

    tokio::select! {
        result = &mut server => result?,
        () = &mut signal => {
            if let Ok(result) = timeout(config.shutdown_grace, &mut server).await {
                result?;
            } else {
                warn!(
                        grace_seconds = config.shutdown_grace.as_secs(),
                        "shutdown grace expired; interrupted work is being stopped with the process"
                    );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use combat_types::{
        BattleType, CombatOutcome, CombatReport, CombatResults, DebrisField, EconomicSummary,
        FleetSnapshot, Participant, PlanetResources, ResourceCost, Technology,
    };
    use std::collections::HashMap;
    use std::sync::{Barrier, Mutex};
    use tower::ServiceExt;

    struct BarrierRunner {
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
        calls: Arc<Mutex<u32>>,
    }

    impl SimulationRunner for BarrierRunner {
        fn run(&self, _request: CombatRequest) -> SimulationResponse {
            let call = {
                let mut calls = self.calls.lock().expect("calls lock");
                *calls += 1;
                *calls
            };
            if call == 1 {
                self.entered.wait();
                self.release.wait();
            }
            SimulationResponse {
                results: CombatResults::new(1),
                report: test_report(),
            }
        }
    }

    fn test_report() -> CombatReport {
        let empty = FleetSnapshot {
            ships: HashMap::default(),
            total_value: ResourceCost::default(),
        };
        CombatReport {
            battle_id: "test".to_owned(),
            timestamp: 0,
            battle_type: BattleType::FleetVsFleet,
            attacker: Participant {
                name: "test".to_owned(),
                player_id: None,
                coordinates: None,
                technology: Technology::default(),
                alliance: None,
            },
            defender: Participant {
                name: "test".to_owned(),
                player_id: None,
                coordinates: None,
                technology: Technology::default(),
                alliance: None,
            },
            outcome: CombatOutcome::Draw,
            rounds: 0,
            attacker_fleet_start: empty.clone(),
            defender_fleet_start: empty.clone(),
            attacker_losses: empty.clone(),
            defender_losses: empty.clone(),
            attacker_fleet_end: empty.clone(),
            defender_fleet_end: empty,
            economics: EconomicSummary {
                debris_field: DebrisField::default(),
                moon_chance: 0.0,
                plunder: PlanetResources::default(),
                attacker_losses_cost: ResourceCost::default(),
                defender_losses_cost: ResourceCost::default(),
                attacker_profit: 0,
                defender_profit: 0,
                harvest_info: None,
            },
            round_details: None,
            moon_destruction: None,
            simulation_count: 1,
            duration_ms: 0,
        }
    }

    fn request() -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/simulate")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"attacker":{"technology":{},"entities":{"204":1}},"defender":{"technology":{},"entities":{"401":1}},"use_rapid_fire":true,"simulations":1}"#,
            ))
            .expect("request")
    }

    #[tokio::test]
    async fn disconnected_admitted_work_keeps_capacity_until_runner_returns() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let calls = Arc::new(Mutex::new(0));
        let runner = Arc::new(BarrierRunner {
            entered: entered.clone(),
            release: release.clone(),
            calls: calls.clone(),
        });
        let config = ServerConfig {
            port: 0,
            max_simulations: 1,
            max_concurrent_simulations: 1,
            shutdown_grace: Duration::from_secs(1),
        };
        let state = AppState::with_runner(config, runner);
        let service = app(state.clone());
        let first = tokio::spawn(service.clone().oneshot(request()));
        tokio::task::spawn_blocking(move || entered.wait())
            .await
            .expect("entered barrier");
        first.abort();

        let second = service.clone().oneshot(request()).await.expect("response");
        assert_eq!(second.status(), StatusCode::SERVICE_UNAVAILABLE);
        let health = service
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).expect("request"))
            .await
            .expect("health response");
        assert_eq!(health.status(), StatusCode::OK);

        tokio::task::spawn_blocking(move || release.wait())
            .await
            .expect("release barrier");
        while state.permits.available_permits() == 0 {
            tokio::task::yield_now().await;
        }
        // The worker drops its permit after the barrier, making the next
        // request admissible without any timing based sleep.
        let third = service.oneshot(request()).await.expect("response");
        assert_eq!(third.status(), StatusCode::OK);
    }
}
