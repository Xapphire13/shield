use std::sync::Arc;

use axum::http::{Request, Response, header::CONTENT_TYPE};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{Level, info, trace};
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt};
use unifi_protect_client::UnifiProtectClient;

use crate::{config::Config, refresh_token_store::RefreshTokenStore};

mod app_error;
mod config;
mod handlers;
mod middleware;
mod refresh_token_store;
mod routes;

#[derive(Clone)]
struct AppState {
    client: Arc<UnifiProtectClient>,
    config: Arc<Config>,
    refresh_token_store: Arc<RefreshTokenStore>,
}

const PORT: i32 = 3000;

#[tokio::main]
async fn main() {
    let filter = filter::Targets::new().with_target("shield_service", Level::TRACE);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    let config = Config::load();

    let app_state = AppState {
        client: Arc::new(UnifiProtectClient::new(
            "https://192.168.1.1",
            &config.credentials.username,
            &config.credentials.password,
        )),
        config: Arc::new(config),
        refresh_token_store: Arc::new(RefreshTokenStore::new()),
    };

    let app = routes::create_routes()
        .with_state(app_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers([CONTENT_TYPE]),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                    )
                })
                .on_request(|_request: &Request<_>, _span: &tracing::Span| {
                    trace!("request started");
                })
                .on_response(
                    |response: &Response<_>,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        trace!(
                            status = response.status().as_u16(),
                            latency_ms = latency.as_millis(),
                            "request completed"
                        );
                    },
                ),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{PORT}"))
        .await
        .unwrap();
    info!("Listening on port {PORT}");
    axum::serve(listener, app).await.unwrap();
}
