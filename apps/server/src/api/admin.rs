use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;

use crate::{
    simulation::{DURABLE_PLAYER_SCHEMA_VERSION, PROTOCOL_VERSION},
    AppState,
};

#[derive(Serialize)]
struct OverviewResponse {
    service: &'static str,
    status: &'static str,
    protocol_version: u16,
    durable_schema_version: u16,
    tick_rate: u32,
    content_releases: Vec<&'static str>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/overview", get(overview))
}

async fn overview(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                "Basic realm=\"Evil Hunter Admin\"",
            )],
            Json(serde_json::json!({"error": "admin_auth_required"})),
        );
    }

    (
        StatusCode::OK,
        [(
            header::WWW_AUTHENTICATE,
            "Basic realm=\"Evil Hunter Admin\"",
        )],
        Json(serde_json::json!(OverviewResponse {
            service: "evil-hunter-admin",
            status: "ok",
            protocol_version: PROTOCOL_VERSION,
            durable_schema_version: DURABLE_PLAYER_SCHEMA_VERSION,
            tick_rate: state.config.simulation.tick_rate,
            content_releases: vec![
                "evil-hunter-1.411.buildings-v1",
                "migration.hunter-demo-v1",
                "evil-hunter-1.411.hunter-info-v1",
            ],
        })),
    )
}

fn authorized(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return false;
    };
    let Ok(decoded) = STANDARD.decode(encoded) else {
        return false;
    };
    let Ok(credentials) = std::str::from_utf8(&decoded) else {
        return false;
    };
    let Some((username, password)) = credentials.split_once(':') else {
        return false;
    };
    constant_time_eq(username.as_bytes(), state.config.admin.username.as_bytes())
        & constant_time_eq(password.as_bytes(), state.config.admin.password.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app_for_test, config::AppConfig};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn admin_requires_basic_authentication() {
        let response = app_for_test(AppConfig::for_test())
            .oneshot(Request::get("/admin/overview").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().contains_key(header::WWW_AUTHENTICATE));
    }

    #[tokio::test]
    async fn admin_returns_overview_for_valid_credentials() {
        let credentials = STANDARD.encode("admin:test-password");
        let response = app_for_test(AppConfig::for_test())
            .oneshot(
                Request::get("/admin/overview")
                    .header(header::AUTHORIZATION, format!("Basic {credentials}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16_384).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["service"], "evil-hunter-admin");
    }
}
