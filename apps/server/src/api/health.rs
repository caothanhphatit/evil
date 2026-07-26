use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    service: &'static str,
    tick_rate: u32,
    dependencies: DependencyStatus,
}

#[derive(Serialize)]
struct DependencyStatus {
    postgres_configured: bool,
    redis_configured: bool,
    postgres_ready: Option<bool>,
    redis_ready: Option<bool>,
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(response(&state, None, None))
}

// Connectivity checks can replace this shallow readiness response once persistence adapters land.
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let postgres_ready = state.repository.is_ready().await;
    let redis_ready = state.coordinator.is_ready().await;
    let mut response = response(&state, Some(postgres_ready), Some(redis_ready));
    let status = if postgres_ready && redis_ready {
        StatusCode::OK
    } else {
        response.status = "unavailable";
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(response))
}

fn response(
    state: &AppState,
    postgres_ready: Option<bool>,
    redis_ready: Option<bool>,
) -> HealthResponse {
    HealthResponse {
        status: "ok",
        service: "evil-hunter-server",
        tick_rate: state.config.simulation.tick_rate,
        dependencies: DependencyStatus {
            postgres_configured: state.config.database_url.is_some(),
            redis_configured: state.config.redis_url.is_some(),
            postgres_ready,
            redis_ready,
        },
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::{app_for_test, config::AppConfig};

    #[tokio::test]
    async fn health_returns_server_status() {
        let response = app_for_test(AppConfig::for_test())
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16_384).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["tick_rate"], 10);
    }
}
