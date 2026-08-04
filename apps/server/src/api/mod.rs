mod admin;
mod admin_security;
mod health;
mod session;
mod websocket;

use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, post},
    Router,
};
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    let request_id_header = header::HeaderName::from_static("x-request-id");
    let configured_origin = state.config.web_origin.clone();
    let loopback_origin = HeaderValue::from_static("http://127.0.0.1:5173");
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            move |origin, _| origin == configured_origin || origin == loopback_origin,
        ))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-csrf-token"),
            header::HeaderName::from_static("x-request-id"),
        ]);

    Router::new()
        .nest("/admin", admin::router(state.clone()))
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .route("/account/register", post(session::register))
        .route("/account/login", post(session::login))
        .route("/session/bootstrap", post(session::bootstrap))
        .route("/session/demo/hunter-lab", get(session::hunter_lab_demo))
        .route("/ws", get(websocket::upgrade))
        .with_state(state)
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
