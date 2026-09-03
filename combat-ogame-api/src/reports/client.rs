use super::{Candidate, MAX_REPORT_BYTES, ReportError, ReportId, parse_report};
use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Fixed-host, non-retaining client. Calling `fetch` deliberately transfers the
/// report ID to a third-party caching proxy. No automatic retries or redirects.
pub struct ReportClient {
    http: reqwest::Client,
    endpoint: reqwest::Url,
}

impl ReportClient {
    pub fn new() -> Result<Self, ReportError> {
        Self::with_endpoint("https://ogapi.faw-kes.de/", Duration::from_secs(20))
    }

    // Private: production callers cannot choose an arbitrary fetch destination.
    pub(super) fn with_endpoint(endpoint: &str, timeout: Duration) -> Result<Self, ReportError> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("ogame-combat-sim/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(timeout)
            .build()
            .map_err(|_| ReportError::Transport)?;
        let endpoint = reqwest::Url::parse(endpoint).map_err(|_| ReportError::Transport)?;
        Ok(Self { http, endpoint })
    }

    /// Fetch one supplied report into a sanitized candidate. Raw bytes are kept
    /// in memory only. Errors deliberately discard URL-bearing transport details.
    pub async fn fetch(&self, id: &ReportId) -> Result<Candidate, ReportError> {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .map_err(|()| ReportError::Transport)?
            .clear()
            .extend(["v1", "report", id.value.as_str(), "1"]);
        reserve_request().await?;
        let mut response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|error| transport(&error))?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after_seconds = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(60)
                .max(60);
            return Err(ReportError::RateLimited {
                retry_after_seconds,
            });
        }
        if !response.status().is_success() {
            return Err(ReportError::HttpStatus(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_REPORT_BYTES as u64)
        {
            return Err(ReportError::TooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| transport(&error))? {
            if chunk.len() > MAX_REPORT_BYTES - bytes.len() {
                return Err(ReportError::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let json = std::str::from_utf8(&bytes).map_err(|_| ReportError::Malformed)?;
        parse_report(id, json)
    }
}

async fn reserve_request() -> Result<(), ReportError> {
    static STARTS: OnceLock<Mutex<VecDeque<Instant>>> = OnceLock::new();
    let mut starts = STARTS
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .await;
    let now = Instant::now();
    let window = Duration::from_secs(60);
    while starts
        .front()
        .is_some_and(|start| now.duration_since(*start) >= window)
    {
        starts.pop_front();
    }
    if starts.len() >= 10 {
        let remaining = starts.front().map_or(window, |start| {
            window.saturating_sub(now.duration_since(*start))
        });
        return Err(ReportError::RateLimited {
            retry_after_seconds: remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0),
        });
    }
    starts.push_back(now);
    Ok(())
}

fn transport(error: &reqwest::Error) -> ReportError {
    if error.is_timeout() {
        ReportError::Timeout
    } else {
        ReportError::Transport
    }
}
