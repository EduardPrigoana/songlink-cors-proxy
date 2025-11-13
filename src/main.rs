use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use lru::LruCache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

const CACHE_SIZE: usize = 1000;
const CACHE_TTL: Duration = Duration::from_secs(2_592_000);

#[derive(Clone)]
struct CacheEntry {
    data: Arc<serde_json::Value>,
    expires_at: Instant,
}

struct AppState {
    client: Client,
    cache: Mutex<LruCache<String, CacheEntry>>,
}

#[derive(Deserialize)]
struct ProxyQuery {
    url: String,
    #[serde(rename = "userCountry")]
    user_country: Option<String>,
    #[serde(rename = "songIfSingle")]
    song_if_single: Option<bool>,
    platform: Option<String>,
    #[serde(rename = "type")]
    entity_type: Option<String>,
    id: Option<String>,
    key: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    status: u16,
}

impl AppState {
    fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .gzip(true)
                .brotli(true)
                .deflate(true)
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .http2_keep_alive_interval(Some(Duration::from_secs(30)))
                .http2_keep_alive_timeout(Duration::from_secs(20))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(CACHE_SIZE).unwrap(),
            )),
        }
    }

    fn normalize_url(url: &str) -> String {
        let replacements = [
            ("monochrome.tf/#", "listen.tidal.com"),
            ("monochrome.prigoana.com/#", "listen.tidal.com"),
            ("tidal.squid.wtf", "listen.tidal.com"),
            ("tidal.qqdl.site", "listen.tidal.com"),
        ];
        
        let mut normalized = url.to_string();
        
        for (from, to) in &replacements {
            if normalized.contains(from) {
                normalized = normalized.replace(from, to);
                break;
            }
        }
        
        normalized
    }

    fn build_api_url(params: &ProxyQuery) -> String {
        let mut api_url = String::with_capacity(256);
        api_url.push_str("https://api.song.link/v1-alpha.1/links?url=");
        api_url.push_str(&urlencoding::encode(&params.url));

        if let Some(country) = &params.user_country {
            api_url.push_str("&userCountry=");
            api_url.push_str(country);
        }

        if let Some(song_if_single) = params.song_if_single {
            api_url.push_str("&songIfSingle=");
            api_url.push_str(if song_if_single { "true" } else { "false" });
        }

        if let Some(platform) = &params.platform {
            api_url.push_str("&platform=");
            api_url.push_str(platform);
        }

        if let Some(entity_type) = &params.entity_type {
            api_url.push_str("&type=");
            api_url.push_str(entity_type);
        }

        if let Some(id) = &params.id {
            api_url.push_str("&id=");
            api_url.push_str(id);
        }

        if let Some(key) = &params.key {
            api_url.push_str("&key=");
            api_url.push_str(key);
        }

        api_url
    }
}

async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    Query(mut params): Query<ProxyQuery>,
) -> Result<Json<serde_json::Value>, Response> {
    params.url = AppState::normalize_url(&params.url);
    
    let cache_key = AppState::build_api_url(&params);

    {
        let mut cache = state.cache.lock().await;
        if let Some(entry) = cache.get(&cache_key) {
            if entry.expires_at > Instant::now() {
                return Ok(Json((*entry.data).clone()));
            }
        }
    }

    let response = state
        .client
        .get(&cache_key)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Failed to fetch from Songlink API: {}", e),
                    status: 502,
                }),
            )
                .into_response()
        })?;

    let status = response.status();

    let json: serde_json::Value = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: format!("Failed to parse response: {}", e),
                status: 502,
            }),
        )
            .into_response()
    })?;

    if status.is_success() {
        let arc_json = Arc::new(json.clone());
        let mut cache = state.cache.lock().await;
        cache.put(
            cache_key,
            CacheEntry {
                data: arc_json,
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
        Ok(Json(json))
    } else {
        Err((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(json),
        )
            .into_response())
    }
}

async fn root_redirect() -> Response {
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(axum::http::header::LOCATION, "https://monochrome.tf")],
    )
        .into_response()
}

async fn health_check() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers(Any);

    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/health", get(health_check))
        .route("/api/links", get(proxy_handler))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("Songlink CORS Proxy running on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
