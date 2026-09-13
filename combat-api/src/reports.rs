//! Bounded, non-retaining adapter over the existing sanitized report client.
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use combat_ogame_api::reports::{Candidate, ReportClient, ReportError, ReportId};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub(super) struct ReportState {
    client: Arc<Result<ReportClient, ReportError>>,
    permits: Arc<Semaphore>,
}
impl ReportState {
    pub(super) fn new() -> Self {
        Self {
            client: Arc::new(ReportClient::new()),
            permits: Arc::new(Semaphore::new(2)),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportRequest {
    key: String,
    consent: bool,
}

pub(super) fn routes() -> Router<super::AppState> {
    Router::new()
        .route("/api/reports/import", post(import))
        .layer(DefaultBodyLimit::max(1024))
}

async fn import(
    State(state): State<super::AppState>,
    request: Result<Json<ImportRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    // Never echo deserialization errors: the rejected body may contain a capability.
    let result = match request {
        Ok(Json(request)) => fetch(&state, request).await,
        Err(_) => Err((StatusCode::BAD_REQUEST, "Ungültige Importanfrage.")),
    };
    let mut response = match result {
        Ok(candidate) => Json(candidate).into_response(),
        Err(error) => error.into_response(),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}
async fn fetch(
    state: &super::AppState,
    request: ImportRequest,
) -> Result<Candidate, (StatusCode, &'static str)> {
    if !request.consent {
        return Err((
            StatusCode::BAD_REQUEST,
            "Bitte der Übertragung an ogapi.faw-kes.de zustimmen.",
        ));
    }
    let id = ReportId::parse(&request.key).map_err(|error| redact(&error))?;
    if !state.is_accepting() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Server wird beendet. Bitte später versuchen.",
        ));
    }
    let _permit = state.reports.permits.try_acquire().map_err(|_| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Import ausgelastet. Bitte später versuchen.",
        )
    })?;
    let client = state.reports.client.as_ref().as_ref().map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Reportimport derzeit nicht verfügbar.",
        )
    })?;
    let candidate = client.fetch(&id).await.map_err(|error| redact(&error))?;
    if candidate.defenders.len() != 1 || candidate.attackers.len() > 1 {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Mehrere Teilnehmer können noch nicht eindeutig zugeordnet werden. Planung bleibt erhalten.",
        ));
    }
    Ok(candidate)
}
fn redact(error: &ReportError) -> (StatusCode, &'static str) {
    match error {
        ReportError::InvalidId | ReportError::UnsupportedKind => (
            StatusCode::BAD_REQUEST,
            "Ungültiger Schlüssel. Einen vollständigen sr- oder cr-Berichtsschlüssel eingeben, keine URL.",
        ),
        ReportError::RateLimited { .. } => (
            StatusCode::TOO_MANY_REQUESTS,
            "Proxy-Quote erreicht. Mindestens eine Minute warten; bei erneutem Fehler später versuchen.",
        ),
        ReportError::Provider | ReportError::HttpStatus(_) => (
            StatusCode::BAD_GATEWAY,
            "Bericht abgelaufen oder nicht verfügbar. Schlüssel prüfen oder einen neuen Bericht verwenden.",
        ),
        ReportError::Timeout | ReportError::Transport => (
            StatusCode::BAD_GATEWAY,
            "Proxy nicht erreichbar oder Zeitlimit überschritten. Bitte später versuchen.",
        ),
        ReportError::Malformed | ReportError::Field(_) | ReportError::TooLarge => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "Berichtsinhalt passt nicht zum Schlüsseltyp oder wird nicht unterstützt.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use std::time::Duration;
    use tower::ServiceExt;

    async fn provider_reply(axum::extract::Path(key): axum::extract::Path<String>) -> Response {
        if key.ends_with('1') {
            return (StatusCode::NOT_FOUND, "private-token").into_response();
        }
        if key.ends_with('2') {
            return (StatusCode::TOO_MANY_REQUESTS, "private-token").into_response();
        }
        if key.ends_with('3') {
            return Json(serde_json::json!({"RESULT_CODE":1000,"RESULT_DATA":{
                "generic":{},"attackers":[{},{}],"defenders":[{}]
            }}))
            .into_response();
        }
        assert!(key.ends_with("en-1-0000000000000000000000000000000000000000"));
        Json(serde_json::json!({"RESULT_CODE":1000,"RESULT_DATA":{
            "generic":{"failed_ships":false,"failed_defense":true,"failed_research":false,"player_name":"private-name"},
            "details":{"ships":[{"ship_type":204,"count":12}],"defense":[{"defense_type":401,"count":999}],"research":[{"research_type":109,"level":8}]}
        }})).into_response()
    }

    fn cases() -> Vec<(serde_json::Value, StatusCode)> {
        [
            ("private-token", true, StatusCode::BAD_REQUEST),
            (
                "sr-en-1-0000000000000000000000000000000000000000",
                false,
                StatusCode::BAD_REQUEST,
            ),
            (
                "sr-en-1-0000000000000000000000000000000000000000",
                true,
                StatusCode::OK,
            ),
            (
                "cr-en-1-0000000000000000000000000000000000000000",
                true,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "sr-en-1-0000000000000000000000000000000000000001",
                true,
                StatusCode::BAD_GATEWAY,
            ),
            (
                "sr-en-1-0000000000000000000000000000000000000002",
                true,
                StatusCode::TOO_MANY_REQUESTS,
            ),
            (
                "cr-en-1-0000000000000000000000000000000000000003",
                true,
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
        ]
        .into_iter()
        .map(|(key, consent, status)| (serde_json::json!({"key":key,"consent":consent}), status))
        .collect()
    }

    #[tokio::test]
    async fn router_uses_real_client_and_sanitizer_without_simulating() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let provider =
            Router::new().route("/v1/report/{key}/1", axum::routing::get(provider_reply));
        let server = tokio::spawn(async move {
            axum::serve(listener, provider).await.unwrap();
        });
        let config = super::super::ServerConfig {
            port: 0,
            max_simulations: 1000,
            max_concurrent_simulations: 1,
            shutdown_grace: Duration::from_secs(1),
        };
        let mut state = super::super::AppState::new(config);
        state.reports.client = Arc::new(ReportClient::for_local_provider(address));
        let service = super::super::app(state.clone());
        for (body, expected) in cases() {
            let response = service
                .clone()
                .oneshot(
                    Request::post("/api/reports/import")
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
            let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
            let text = std::str::from_utf8(&bytes).unwrap();
            assert!(!text.contains("private-"));
            assert!(!text.contains("0000000000000000000000000000000000000000"));
            if expected == StatusCode::OK {
                let candidate: Candidate = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(candidate.defenders[0].ships.as_ref().unwrap()[&204], 12);
                assert!(candidate.defenders[0].defenses.is_none());
                assert!(candidate.defenders[0].entities.is_none());
                assert_eq!(candidate.defenders[0].technology.weapon, Some(8));
                assert_eq!(candidate.defenders[0].technology.shield, None);
            }
        }
        assert_eq!(
            state
                .workers
                .active
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );
        server.abort();
    }
}
