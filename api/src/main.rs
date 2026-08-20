//! TarascaPay — API HTTP.

mod auth;
mod config;
mod error;
mod models;
mod repo;
mod routes;
mod state;
mod validate;

use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    init_tracing();

    if let Err(err) = run().await {
        tracing::error!("{err}");
        std::process::exit(1);
    }
}

fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("api=info,tower_http=info,sqlx=warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn run() -> Result<(), String> {
    let config = Config::from_env()?;

    tokio::fs::create_dir_all(&config.upload_dir)
        .await
        .map_err(|e| {
            format!(
                "no se pudo crear el directorio de subidas {}: {e}",
                config.upload_dir.display()
            )
        })?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect(&config.database_url)
        .await
        .map_err(|e| format!("no se pudo conectar a la base de datos: {e}"))?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .map_err(|e| format!("fallaron las migraciones: {e}"))?;

    tracing::info!("migraciones al día");

    let addr = format!("{}:{}", config.host, config.port);
    let max_body = config.max_upload_bytes + 1024 * 1024;
    let cors = build_cors(&config)?;
    let web_dist = config.web_dist.clone();

    let state = AppState::new(db, config);

    let mut app = Router::new()
        .nest("/api", routes::api_router())
        .layer(DefaultBodyLimit::max(max_body))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // En producción el mismo binario puede servir el SPA compilado.
    // Cualquier ruta desconocida cae en index.html para que funcione el
    // enrutado del lado del cliente.
    if let Some(dist) = web_dist {
        if dist.is_dir() {
            let index = dist.join("index.html");
            app = app.fallback_service(ServeDir::new(&dist).fallback(ServeFile::new(index)));
            tracing::info!("sirviendo el frontend desde {}", dist.display());
        } else {
            tracing::warn!("WEB_DIST apunta a {} y no existe", dist.display());
            app = app.fallback(not_found);
        }
    } else {
        app = app.fallback(not_found);
    }

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("no se pudo escuchar en {addr}: {e}"))?;

    tracing::info!("TarascaPay API escuchando en http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| format!("el servidor terminó con error: {e}"))
}

fn build_cors(config: &Config) -> Result<CorsLayer, String> {
    let layer = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(Duration::from_secs(3600));

    if config.cors_origins.iter().any(|o| o == "*") {
        // No usamos cookies, así que un origen abierto es aceptable en
        // desarrollo. En producción conviene listar los dominios.
        return Ok(layer.allow_origin(AllowOrigin::any()));
    }

    let origins = config
        .cors_origins
        .iter()
        .map(|o| {
            o.parse::<HeaderValue>()
                .map_err(|_| format!("origen CORS inválido: '{o}'"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(layer.allow_origin(AllowOrigin::list(origins)))
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        axum::Json(shared::ErrorResponse {
            error: "ruta no encontrada".to_string(),
        }),
    )
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("no se pudo instalar el manejador de Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("no se pudo instalar el manejador de SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("apagando el servidor…");
}
