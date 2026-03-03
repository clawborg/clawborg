use crate::cache;
use crate::routes;
use crate::types::AppState;
use crate::watcher;
use crate::ws;
use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use rust_embed::Embed;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Embedded frontend assets (compiled React build)
/// In development, this folder may be empty — ClawBorg serves API only.
/// In release, `cargo build --release` embeds the pre-built web/ dist.
#[derive(Embed)]
#[folder = "../../web/dist"]
#[prefix = ""]
struct FrontendAssets;

pub struct ServerConfig {
    pub port: u16,
    pub openclaw_dir: PathBuf,
    pub watch_enabled: bool,
    pub readonly: bool,
}

pub async fn run(config: ServerConfig) -> anyhow::Result<()> {
    let (file_events_tx, _) = broadcast::channel::<crate::types::FileChangeEvent>(256);
    let clawborg_config = crate::clawborg_config::load();

    // Load initial cache from disk
    let initial_cache = {
        let agents = crate::openclaw::config::read_config(&config.openclaw_dir)
            .map(|cfg| crate::openclaw::config::resolve_agents(&cfg, &config.openclaw_dir))
            .unwrap_or_default();
        cache::load_cache(&agents, &config.openclaw_dir)
    };
    let app_cache = Arc::new(RwLock::new(initial_cache));

    let state = AppState {
        openclaw_dir: config.openclaw_dir.clone(),
        readonly: config.readonly,
        file_events_tx: file_events_tx.clone(),
        clawborg_config,
        cache: app_cache.clone(),
    };

    // Start file watcher if enabled
    if config.watch_enabled {
        let watcher_dir = config.openclaw_dir.clone();
        let watcher_tx = file_events_tx.clone();
        let watcher_cache = app_cache.clone();
        tokio::spawn(async move {
            if let Err(e) = watcher::start_watching(watcher_dir, watcher_tx, watcher_cache).await {
                tracing::error!("File watcher error: {e}");
            }
        });
        tracing::info!("👁️ File watcher started");
    }

    // Build API router
    let api = Router::new()
        .route("/agents", get(routes::agents::list_agents))
        .route("/agents/{id}", get(routes::agents::get_agent))
        .route(
            "/agents/{id}/files/{*filename}",
            get(routes::files::get_file).put(routes::files::update_file),
        )
        .route("/agents/{id}/browse", get(routes::agents::browse_agent))
        .route("/agents/{id}/tasks", get(routes::tasks::list_tasks))
        .route("/sessions", get(routes::sessions::list_sessions))
        .route("/health", get(routes::health::health_audit))
        .route("/config", get(routes::config::get_config))
        // v0.2 endpoints
        .route("/usage", get(routes::usage::get_usage))
        .route("/crons", get(routes::crons::list_crons))
        .route("/alerts", get(routes::alerts::get_alerts));

    let app = Router::new()
        .nest("/api", api)
        .route("/ws", get(ws::ws_handler))
        // Serve embedded frontend for all non-API routes
        .fallback(serve_frontend)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("🚀 Server listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Serve embedded frontend assets (SPA with index.html fallback)
async fn serve_frontend(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    // Try exact file first
    if let Some(file) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CACHE_CONTROL, cache_control(path))
            .body(Body::from(file.data.to_vec()))
            .unwrap();
    }

    // SPA fallback: serve index.html for all unmatched routes
    match FrontendAssets::get("index.html") {
        Some(index) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(index.data.to_vec()))
            .unwrap(),
        None => {
            // No embedded frontend (dev mode)
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(Body::from(dev_mode_html()))
                .unwrap()
        }
    }
}

/// Cache control headers for embedded assets
fn cache_control(path: &str) -> &'static str {
    if path.contains(".js") || path.contains(".css") {
        "public, max-age=31536000, immutable"
    } else if path.contains(".woff") || path.contains(".ttf") || path.contains(".svg") {
        "public, max-age=86400"
    } else {
        "no-cache"
    }
}

/// Development mode HTML when no frontend is embedded
fn dev_mode_html() -> String {
    r#"<!DOCTYPE html>
<html>
<head><title>ClawBorg — Dev Mode</title>
<style>
  body { font-family: system-ui; background: #0a0a0b; color: #e4e4e7; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; }
  .box { text-align: center; max-width: 480px; }
  h1 { font-size: 2rem; margin-bottom: 0.5rem; }
  code { background: #27272a; padding: 2px 8px; border-radius: 4px; font-size: 0.9rem; }
  .api { margin-top: 2rem; padding: 1rem; background: #18181b; border-radius: 8px; }
  a { color: #60a5fa; }
</style>
</head>
<body>
<div class="box">
  <h1>ClawBorg</h1>
  <p>No embedded frontend found. Run the dev server:</p>
  <p><code>cd web && pnpm dev</code></p>
  <div class="api">
    <p>API is live:</p>
    <p><a href="/api/agents">/api/agents</a> · <a href="/api/usage">/api/usage</a> · <a href="/api/crons">/api/crons</a> · <a href="/api/alerts">/api/alerts</a></p>
  </div>
</div>
</body>
</html>"#.to_string()
}
