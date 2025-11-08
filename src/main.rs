use axum::{
    extract::{Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rand::seq::SliceRandom;
use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    client: Client,
    user_agents: Vec<String>,
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
        let user_agents = vec![
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15".to_string(),
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15".to_string(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0".to_string(),
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        ];

        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .gzip(true)
                .brotli(true)
                .deflate(true)
                .build()
                .expect("Failed to create HTTP client"),
            user_agents,
        }
    }

    fn get_random_user_agent(&self) -> &str {
        self.user_agents
            .choose(&mut rand::thread_rng())
            .map(|s| s.as_str())
            .unwrap_or("Mozilla/5.0")
    }

    fn build_request_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        
        let accept_options = [
            "application/json, text/plain, */*",
            "application/json",
            "*/*",
        ];
        let accept = accept_options.choose(&mut rand::thread_rng()).unwrap();
        headers.insert("Accept", HeaderValue::from_static(accept));
        
        let accept_lang_options = [
            "en-US,en;q=0.9",
            "en-US,en;q=0.9,es;q=0.8",
            "en-GB,en;q=0.9",
            "en-US,en;q=0.8",
        ];
        let accept_lang = accept_lang_options.choose(&mut rand::thread_rng()).unwrap();
        headers.insert("Accept-Language", HeaderValue::from_static(accept_lang));
        
        let accept_encoding_options = [
            "gzip, deflate, br",
            "gzip, deflate",
            "gzip",
        ];
        let accept_encoding = accept_encoding_options.choose(&mut rand::thread_rng()).unwrap();
        headers.insert("Accept-Encoding", HeaderValue::from_static(accept_encoding));
        
        if rand::random::<bool>() {
            headers.insert("DNT", HeaderValue::from_static("1"));
        }
        
        let connection_options = ["keep-alive", "close"];
        let connection = connection_options.choose(&mut rand::thread_rng()).unwrap();
        headers.insert("Connection", HeaderValue::from_static(connection));
        
        headers
    }
}

async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProxyQuery>,
) -> Result<Json<serde_json::Value>, Response> {
    let mut api_url = "https://api.song.link/v1-alpha.1/links?".to_string();
    
    if !params.url.is_empty() {
        api_url.push_str(&format!("url={}", urlencoding::encode(&params.url)));
    }
    
    if let Some(country) = params.user_country {
        api_url.push_str(&format!("&userCountry={}", country));
    }
    
    if let Some(song_if_single) = params.song_if_single {
        api_url.push_str(&format!("&songIfSingle={}", song_if_single));
    }
    
    if let Some(platform) = params.platform {
        api_url.push_str(&format!("&platform={}", platform));
    }
    
    if let Some(entity_type) = params.entity_type {
        api_url.push_str(&format!("&type={}", entity_type));
    }
    
    if let Some(id) = params.id {
        api_url.push_str(&format!("&id={}", id));
    }
    
    if let Some(key) = params.key {
        api_url.push_str(&format!("&key={}", key));
    }
    
    let mut headers = state.build_request_headers();
    let user_agent = state.get_random_user_agent();
    headers.insert("User-Agent", HeaderValue::from_str(user_agent).unwrap());
    
    let response = state
        .client
        .get(&api_url)
        .headers(headers)
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
        Ok(Json(json))
    } else {
        Err((StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(json)).into_response())
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
    
    let is_dev = std::env::var("DEV")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);
    
    let cors = if is_dev {
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(|origin: &axum::http::HeaderValue, _| {
                origin.to_str()
                    .map(|o| o.starts_with("http://localhost") || o.starts_with("http://127.0.0.1"))
                    .unwrap_or(false)
            }))
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(tower_http::cors::Any)
            .expose_headers(tower_http::cors::Any)
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list([
                "https://monochrome.tf".parse::<axum::http::HeaderValue>().unwrap(),
                "https://monochrome.prigoana.com".parse::<axum::http::HeaderValue>().unwrap(),
            ]))
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(tower_http::cors::Any)
            .expose_headers(tower_http::cors::Any)
    };
    
    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/health", get(health_check))
        .route("/api/links", get(proxy_handler))
        .layer(cors)
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    
    println!("🚀 Songlink CORS Proxy running on http://0.0.0.0:3000");
    println!("📡 Mode: {}", if is_dev { "DEV (localhost only)" } else { "PROD (monochrome.tf, monochrome.prigoana.com)" });
    
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}