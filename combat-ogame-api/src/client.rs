use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex;

use crate::{
    Endpoint, Error, Highscore, HighscoreCategory, HighscoreType, PlayerData, Players, ServerData,
    Timestamped, Universe, UniverseData, parse_highscore, parse_player_data, parse_players,
    parse_server_data, parse_universe,
};

const REQUEST_INTERVAL: Duration = Duration::from_secs(1);
const CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);
const DEFAULT_CONTACT: &str = "https://github.com/maximalcode/ogame-combat-sim/issues";

static LAST_REQUESTS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

/// Async client for one caller-selected `OGame` universe.
///
/// Clones share reqwest's connection pool and this instance's cache lock. The
/// one-request-per-second limiter is process-wide, so separate clients aimed at
/// the same host still respect Gameforge's limit.
#[derive(Debug, Clone)]
pub struct OGameClient {
    http: reqwest::Client,
    universe: Universe,
    cache_dir: PathBuf,
    base_url: String,
    host_key: String,
    cache_lock: Arc<Mutex<()>>,
}

impl OGameClient {
    /// Build a client with the project's issue tracker as its contact address.
    pub fn new(universe: Universe, cache_dir: impl Into<PathBuf>) -> Result<Self, Error> {
        Self::with_contact(universe, cache_dir, DEFAULT_CONTACT)
    }

    /// Build a client with a caller-supplied contact address in the User-Agent.
    pub fn with_contact(
        universe: Universe,
        cache_dir: impl Into<PathBuf>,
        contact: &str,
    ) -> Result<Self, Error> {
        let host = universe.host();
        Self::build(
            universe,
            cache_dir.into(),
            contact,
            format!("https://{host}"),
            host,
        )
    }

    /// Fetch and parse `serverData.xml`, using the daily disk cache when fresh.
    pub async fn server_data(&self) -> Result<ServerData, Error> {
        self.fetch(Endpoint::ServerData, parse_server_data).await
    }

    /// Fetch and parse `players.xml`, using the daily disk cache when fresh.
    pub async fn players(&self) -> Result<Players, Error> {
        self.fetch(Endpoint::Players, parse_players).await
    }

    /// Fetch and parse `universe.xml`, using the weekly disk cache when fresh.
    pub async fn universe(&self) -> Result<UniverseData, Error> {
        self.fetch(Endpoint::Universe, parse_universe).await
    }

    /// Fetch and parse one `playerData.xml`, cached for one week per player.
    pub async fn player_data(&self, player_id: u64) -> Result<PlayerData, Error> {
        self.fetch(Endpoint::PlayerData { player_id }, parse_player_data)
            .await
    }

    /// Fetch and parse one hourly highscore category/type combination.
    pub async fn highscore(
        &self,
        category: HighscoreCategory,
        score_type: HighscoreType,
    ) -> Result<Highscore, Error> {
        self.fetch(
            Endpoint::Highscore {
                category,
                score_type,
            },
            parse_highscore,
        )
        .await
    }

    fn build(
        universe: Universe,
        cache_dir: PathBuf,
        contact: &str,
        base_url: String,
        host_key: String,
    ) -> Result<Self, Error> {
        validate_contact(contact)?;
        let user_agent = format!(
            "ogame-combat-sim/{} (+{contact})",
            env!("CARGO_PKG_VERSION")
        );
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .build()
            .map_err(Error::BuildClient)?;
        Ok(Self {
            http,
            universe,
            cache_dir,
            base_url,
            host_key,
            cache_lock: Arc::new(Mutex::new(())),
        })
    }

    #[cfg(test)]
    fn for_test(base_url: String, cache_dir: PathBuf) -> Result<Self, Error> {
        let host_key = base_url.clone();
        Self::build(
            Universe::new("s1-en")?,
            cache_dir,
            DEFAULT_CONTACT,
            base_url,
            host_key,
        )
    }

    async fn fetch<T>(
        &self,
        endpoint: Endpoint,
        parser: fn(&str) -> Result<T, Error>,
    ) -> Result<T, Error>
    where
        T: Timestamped,
    {
        let now = unix_now()?;
        {
            let _cache_guard = self.cache_lock.lock().await;
            if let Some(value) = self.read_cache(endpoint, parser, now).await? {
                return Ok(value);
            }
        }

        wait_for_host(&self.host_key).await;
        let url = format!("{}/{}", self.base_url, endpoint.relative_url());
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|source| Error::Request { endpoint, source })?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::HttpStatus { endpoint, status });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|source| Error::Request { endpoint, source })?;
        let xml = String::from_utf8(bytes.to_vec())
            .map_err(|source| Error::Decode { endpoint, source })?;
        let value = parser(&xml)?;
        if !source_is_fresh(value.timestamp(), now, endpoint.ttl()) {
            return Err(Error::StaleResponse {
                endpoint,
                timestamp: value.timestamp(),
            });
        }

        let _cache_guard = self.cache_lock.lock().await;
        self.write_cache(endpoint, &xml).await?;
        Ok(value)
    }

    async fn read_cache<T>(
        &self,
        endpoint: Endpoint,
        parser: fn(&str) -> Result<T, Error>,
        now: Duration,
    ) -> Result<Option<T>, Error>
    where
        T: Timestamped,
    {
        let path = self.cache_path(endpoint);
        let xml = match tokio::fs::read_to_string(&path).await {
            Ok(xml) => xml,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(Error::Cache { endpoint, source }),
        };
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|source| Error::Cache { endpoint, source })?;
        let modified = metadata
            .modified()
            .map_err(|source| Error::Cache { endpoint, source })?
            .duration_since(UNIX_EPOCH)
            .map_err(|source| Error::Clock(io::Error::other(source)))?;
        let Ok(value) = parser(&xml) else {
            return Ok(None);
        };
        Ok(cache_is_fresh(value.timestamp(), modified, now, endpoint.ttl()).then_some(value))
    }

    async fn write_cache(&self, endpoint: Endpoint, xml: &str) -> Result<(), Error> {
        let directory = self.cache_dir.join(self.universe.as_str());
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(|source| Error::Cache { endpoint, source })?;
        let path = directory.join(endpoint.cache_file_name());
        let temporary = path.with_extension(format!("xml.tmp-{}", std::process::id()));
        tokio::fs::write(&temporary, xml)
            .await
            .map_err(|source| Error::Cache { endpoint, source })?;
        if tokio::fs::try_exists(&path)
            .await
            .map_err(|source| Error::Cache { endpoint, source })?
        {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|source| Error::Cache { endpoint, source })?;
        }
        tokio::fs::rename(&temporary, &path)
            .await
            .map_err(|source| Error::Cache { endpoint, source })
    }

    fn cache_path(&self, endpoint: Endpoint) -> PathBuf {
        self.cache_dir
            .join(self.universe.as_str())
            .join(endpoint.cache_file_name())
    }
}

fn validate_contact(contact: &str) -> Result<(), Error> {
    let valid_scheme = contact.starts_with("https://") || contact.starts_with("mailto:");
    if valid_scheme && !contact.bytes().any(|byte| byte.is_ascii_whitespace()) {
        Ok(())
    } else {
        Err(Error::InvalidContact(contact.to_owned()))
    }
}

fn unix_now() -> Result<Duration, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| Error::Clock(io::Error::other(source)))
}

fn cache_is_fresh(
    source_timestamp: u64,
    cached_at: Duration,
    now: Duration,
    ttl: Duration,
) -> bool {
    source_is_fresh(source_timestamp, now, ttl) && now.saturating_sub(cached_at) <= ttl
}

fn source_is_fresh(source_timestamp: u64, now: Duration, ttl: Duration) -> bool {
    let source = Duration::from_secs(source_timestamp);
    source <= now + CLOCK_SKEW && now.saturating_sub(source) <= ttl
}

async fn wait_for_host(host: &str) {
    let requests = LAST_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()));
    let wait = {
        let mut requests = requests.lock().await;
        let now = Instant::now();
        let next = requests
            .get(host)
            .copied()
            .map_or(now, |last| (last + REQUEST_INTERVAL).max(now));
        requests.insert(host.to_owned(), next);
        next.saturating_duration_since(now)
    };
    tokio::time::sleep(wait).await;
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    use super::*;

    static TEST_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
            Self(std::env::temp_dir().join(format!("combat-ogame-api-{}-{id}", std::process::id())))
        }
    }

    impl AsRef<Path> for TestDirectory {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn freshness_requires_both_cache_age_and_root_timestamp() {
        let now = Duration::from_secs(10_000);
        let ttl = Duration::from_secs(1_000);

        assert!(cache_is_fresh(9_500, Duration::from_secs(9_500), now, ttl));
        assert!(!cache_is_fresh(8_000, Duration::from_secs(9_500), now, ttl));
        assert!(!cache_is_fresh(9_500, Duration::from_secs(8_000), now, ttl));
    }

    #[test]
    fn contact_address_is_required_and_visible_in_the_user_agent() {
        assert!(validate_contact(DEFAULT_CONTACT).is_ok());
        assert!(validate_contact("mailto:maintainer@example.test").is_ok());
        assert!(validate_contact("anonymous bot").is_err());
    }

    #[tokio::test]
    async fn a_fresh_response_is_read_from_disk_on_the_second_call() {
        let now = unix_now().expect("clock").as_secs();
        let body = format!(
            r#"<players timestamp="{now}" serverId="en1"><player id="7" name="Ada"/></players>"#
        );
        let (base_url, request, server) = serve_once(body).await;
        let cache = TestDirectory::new();
        let client = OGameClient::for_test(base_url, cache.as_ref().to_owned()).expect("client");

        assert_eq!(
            client.players().await.expect("network").players[0].name,
            "Ada"
        );
        server.await.expect("server task");
        assert_eq!(
            client.players().await.expect("cache").players[0].name,
            "Ada"
        );
        assert!(
            request
                .await
                .expect("request capture")
                .to_ascii_lowercase()
                .contains("user-agent: ogame-combat-sim/")
        );
    }

    #[tokio::test]
    async fn an_unreachable_endpoint_names_which_request_failed() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        drop(listener);
        let cache = TestDirectory::new();
        let client = OGameClient::for_test(format!("http://{address}"), cache.as_ref().to_owned())
            .expect("client");

        let error = client.players().await.expect_err("request should fail");

        assert!(error.to_string().contains("players.xml"));
    }

    #[tokio::test]
    async fn requests_to_one_host_are_started_at_most_once_per_second() {
        let host = format!(
            "rate-limit-test-{}",
            TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed)
        );
        wait_for_host(&host).await;

        let started = Instant::now();
        wait_for_host(&host).await;

        assert!(started.elapsed() >= REQUEST_INTERVAL);
    }

    async fn serve_once(
        body: String,
    ) -> (
        String,
        oneshot::Receiver<String>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("address");
        let (request_sender, request_receiver) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buffer = vec![0; 4096];
            let read = stream.read(&mut buffer).await.expect("read request");
            let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
            let _ = request_sender.send(request);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });
        (format!("http://{address}"), request_receiver, server)
    }
}
