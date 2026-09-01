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
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};
use tokio::{
    sync::{Notify, Semaphore, oneshot, watch},
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

struct WorkerTracker {
    active: AtomicUsize,
    completed: Notify,
}

impl WorkerTracker {
    fn new() -> Self {
        Self {
            active: AtomicUsize::new(0),
            completed: Notify::new(),
        }
    }

    fn admitted(&self) {
        self.active.fetch_add(1, Ordering::AcqRel);
    }

    fn returned(&self) {
        self.active.fetch_sub(1, Ordering::AcqRel);
        self.completed.notify_waiters();
    }

    async fn wait_for_all(&self) {
        loop {
            let notified = self.completed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.active.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }
}

struct WorkerGuard(Arc<WorkerTracker>);

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.0.returned();
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
    workers: Arc<WorkerTracker>,
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
            workers: Arc::new(WorkerTracker::new()),
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

    /// Wait for every admitted simulation worker to return.
    pub async fn wait_for_admitted_work(&self) {
        self.workers.wait_for_all().await;
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
    let workers = state.workers.clone();
    workers.admitted();
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
    let worker_for_thread = workers.clone();
    let (sender, receiver) = oneshot::channel();
    thread::Builder::new()
        .name("combat-api-simulation".to_owned())
        .spawn(move || {
            let _worker_guard = WorkerGuard(worker_for_thread);
            let response = runner.run(request);
            // The permit is deliberately dropped only after run returns. If
            // the HTTP receiver was dropped, this still protects capacity.
            drop(permit);
            let _ = sender.send(response);
        })
        .map_err(|error| {
            workers.returned();
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
    let signal = wait_for_shutdown(state.clone(), shutdown_sender);
    tokio::pin!(signal);

    tokio::select! {
        result = &mut server => {
            result?;
            if !state.is_accepting()
                && timeout(config.shutdown_grace, state.wait_for_admitted_work())
                    .await
                    .is_err()
            {
                warn!(
                    grace_seconds = config.shutdown_grace.as_secs(),
                    "shutdown grace expired; interrupted work is being stopped with the process"
                );
            }
        },
        () = &mut signal => {
            let drain = async {
                let result = (&mut server).await;
                state.wait_for_admitted_work().await;
                result
            };
            if let Ok(result) = timeout(config.shutdown_grace, drain).await {
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
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Barrier, Mutex};
    use std::task::{Context, Poll};
    use tokio::sync::oneshot;
    use tower::ServiceExt;

    struct ObservePending<F> {
        future: Pin<Box<F>>,
        observed: Option<oneshot::Sender<bool>>,
    }

    impl<F: Future> Future for ObservePending<F> {
        type Output = F::Output;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            let result = self.future.as_mut().poll(context);
            if let Some(sender) = self.observed.take() {
                let _ = sender.send(result.is_pending());
            }
            result
        }
    }

    struct BarrierRunner {
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
        calls: Arc<Mutex<u32>>,
    }

    struct ReleaseGuard {
        barrier: Arc<Barrier>,
        released: bool,
    }

    impl ReleaseGuard {
        fn new(barrier: Arc<Barrier>) -> Self {
            Self {
                barrier,
                released: false,
            }
        }
    }

    impl Drop for ReleaseGuard {
        fn drop(&mut self) {
            if !self.released {
                self.barrier.wait();
            }
        }
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
        let mut release_guard = ReleaseGuard::new(release.clone());
        first.abort();
        let cancellation = first.await.expect_err("aborted request must be cancelled");
        assert!(cancellation.is_cancelled());

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
        release_guard.released = true;
        while state.permits.available_permits() == 0 {
            tokio::task::yield_now().await;
        }
        // The worker drops its permit after the barrier, making the next
        // request admissible without any timing based sleep.
        let third = service.oneshot(request()).await.expect("response");
        assert_eq!(third.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn shutdown_drain_waits_for_disconnected_worker() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let runner = Arc::new(BarrierRunner {
            entered: entered.clone(),
            release: release.clone(),
            calls: Arc::new(Mutex::new(0)),
        });
        let config = ServerConfig {
            port: 0,
            max_simulations: 1,
            max_concurrent_simulations: 1,
            shutdown_grace: Duration::from_secs(1),
        };
        let state = AppState::with_runner(config, runner);
        let service = app(state.clone());
        let first = tokio::spawn(service.oneshot(request()));
        tokio::task::spawn_blocking(move || entered.wait())
            .await
            .expect("entered barrier");
        first.abort();
        state.stop_admission();

        let (observed_sender, observed_receiver) = oneshot::channel();
        let drain_state = state.clone();
        let drain = tokio::spawn(ObservePending {
            future: Box::pin(async move { drain_state.wait_for_admitted_work().await }),
            observed: Some(observed_sender),
        });
        let pending = observed_receiver.await.expect("drain was polled");
        if !pending {
            release.wait();
            let _ = drain.await;
            panic!("shutdown drain returned before the admitted worker ended");
        }

        release.wait();
        drain.await.expect("drain task");
    }
}
