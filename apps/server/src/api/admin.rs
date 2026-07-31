use axum::{
    extract::{Path, State},
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
    Router::new()
        .route("/overview", get(overview))
        .route("/catalogs", get(catalogs))
        .route("/catalogs/{catalog_id}", get(catalog))
}

const CATALOGS: [(&str, &str, &str); 7] = [
    ("buildings", "Buildings", include_str!("../../../../packages/content/releases/evil-hunter-1.411/building-registry.json")),
    ("experience", "Experience", include_str!("../../../../packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json")),
    ("gear", "Gear", include_str!("../../../../packages/content/releases/evil-hunter-1.411/gear-catalog.json")),
    ("hunter-assets", "Hunter assets", include_str!("../../../../packages/content/releases/evil-hunter-1.411/hunter-assets.json")),
    ("monster-materials", "Monster materials", include_str!("../../../../packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json")),
    ("monsters", "Monsters", include_str!("../../../../packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json")),
    ("world-map", "World map", include_str!("../../../../packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json")),
];

async fn catalogs(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "admin_auth_required"})),
        );
    }
    let catalogs = CATALOGS
        .iter()
        .map(|(id, label, source)| {
            let value: serde_json::Value =
                serde_json::from_str(source).expect("embedded catalog must be valid JSON");
            let mut collections = Vec::new();
            collect_array_collections("", &value, &mut collections);
            serde_json::json!({"id": id, "label": label, "collections": collections})
        })
        .collect::<Vec<_>>();
    (
        StatusCode::OK,
        Json(serde_json::json!({"catalogs": catalogs})),
    )
}

fn collect_array_collections(
    path: &str,
    value: &serde_json::Value,
    output: &mut Vec<serde_json::Value>,
) {
    match value {
        serde_json::Value::Array(rows) => {
            if !path.is_empty() {
                output.push(serde_json::json!({"id": path, "count": rows.len()}));
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, child) in fields {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                collect_array_collections(&child_path, child, output);
            }
        }
        _ => {}
    }
}

async fn catalog(
    Path(catalog_id): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "admin_auth_required"})),
        );
    }
    let Some((_, _, source)) = CATALOGS.iter().find(|(id, _, _)| *id == catalog_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "admin_catalog_not_found"})),
        );
    };
    let value = serde_json::from_str(source).expect("embedded catalog must be valid JSON");
    (StatusCode::OK, Json(value))
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

    #[tokio::test]
    async fn admin_lists_and_returns_embedded_catalogs() {
        let credentials = STANDARD.encode("admin:test-password");
        let app = app_for_test(AppConfig::for_test());
        let response = app
            .clone()
            .oneshot(
                Request::get("/admin/catalogs")
                    .header(header::AUTHORIZATION, format!("Basic {credentials}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::get("/admin/catalogs/gear")
                    .header(header::AUTHORIZATION, format!("Basic {credentials}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = to_bytes(response.into_body(), 2_000_000).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["rows"].as_array().unwrap().len(), 671);
    }
}
