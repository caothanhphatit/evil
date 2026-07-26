mod health;
mod session;
mod websocket;

use axum::{
    http::{header, Method},
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
    let cors = CorsLayer::new()
        .allow_origin(state.config.web_origin.clone())
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);

    Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .route("/session/bootstrap", post(session::bootstrap))
        .route("/session/demo/hunter-lab", get(session::hunter_lab_demo))
        .route("/ws", get(websocket::upgrade))
        .with_state(state)
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
