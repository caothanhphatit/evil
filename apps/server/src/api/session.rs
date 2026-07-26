use std::time::Duration;

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    coordination::CoordinationError, identity::SessionTokenHash, persistence::RepositoryError,
    AppState,
};

pub const SESSION_COOKIE: &str = "eh_session";
pub const HUNTER_LAB_DEMO_SESSION_TOKEN: Uuid = Uuid::from_u128(0x0000000000004000800000000000d001);

#[derive(Serialize)]
struct BootstrapResponse {
    status: &'static str,
}

#[derive(Debug, Error)]
pub enum SessionResolutionError {
    #[error("durable identity lookup failed")]
    Repository(#[from] RepositoryError),
    #[error("session cache failed")]
    Coordination(#[from] CoordinationError),
}

pub async fn bootstrap(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = session_token(&headers).unwrap_or_else(Uuid::new_v4);
    let status = if activate_session(&state, token).await.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    bootstrap_response(&state, token, status)
}

pub async fn hunter_lab_demo(State(state): State<AppState>) -> Response {
    if activate_session(&state, HUNTER_LAB_DEMO_SESSION_TOKEN)
        .await
        .is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let mut response = Redirect::to("/").into_response();
    set_session_cookie(&state, HUNTER_LAB_DEMO_SESSION_TOKEN, &mut response);
    response
}

async fn activate_session(state: &AppState, token: Uuid) -> Result<(), SessionResolutionError> {
    let token_hash = SessionTokenHash::from_token(token);
    let ttl = Duration::from_secs(state.config.session.ttl_seconds);
    let player = state
        .repository
        .resolve_or_create_local_identity(token_hash)
        .await?;
    state
        .coordinator
        .cache_session(token_hash, player, ttl)
        .await?;
    Ok(())
}

pub async fn resolve_player(
    state: &AppState,
    token: Uuid,
    ttl: Duration,
) -> Result<Option<Uuid>, SessionResolutionError> {
    let token_hash = SessionTokenHash::from_token(token);
    if let Some(player) = state.coordinator.resolve(token_hash, ttl).await? {
        return Ok(Some(player));
    }
    let Some(player) = state.repository.resolve_local_identity(token_hash).await? else {
        return Ok(None);
    };
    state
        .coordinator
        .cache_session(token_hash, player, ttl)
        .await?;
    Ok(Some(player))
}

fn bootstrap_response(state: &AppState, token: Uuid, status: StatusCode) -> Response {
    let body_status = if status == StatusCode::OK {
        "ready"
    } else {
        "unavailable"
    };
    let mut response = (
        status,
        Json(BootstrapResponse {
            status: body_status,
        }),
    )
        .into_response();
    set_session_cookie(state, token, &mut response);
    response
}

fn set_session_cookie(state: &AppState, token: Uuid, response: &mut Response) {
    let secure = if state.config.session.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        state.config.session.ttl_seconds, secure
    );
    if let Ok(value) = cookie.parse() {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
}

pub fn session_token(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{SESSION_COOKIE}=")))
        .and_then(|value| Uuid::parse_str(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use std::sync::Arc;

    use crate::{
        app_for_test,
        config::AppConfig,
        coordination::InMemorySessionCoordinator,
        persistence::{InMemoryPlayerRepository, PlayerRepository},
    };

    #[test]
    fn extracts_only_named_session_cookie() {
        let token = Uuid::new_v4();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("theme=dark; {SESSION_COOKIE}={token}; x=y")
                .parse()
                .unwrap(),
        );
        assert_eq!(session_token(&headers), Some(token));
    }

    #[test]
    fn hunter_lab_demo_token_matches_the_seeded_identity() {
        assert_eq!(
            HUNTER_LAB_DEMO_SESSION_TOKEN.to_string(),
            "00000000-0000-4000-8000-00000000d001"
        );
        assert_eq!(
            SessionTokenHash::from_token(HUNTER_LAB_DEMO_SESSION_TOKEN).cache_key_suffix(),
            "19630c7f4811fdf6fe56d1c9978ec156d2b13b1e04f7017bc3a07e347baa943d"
        );
    }

    #[tokio::test]
    async fn bootstrap_sets_http_only_cookie_without_exposing_identity() {
        let response = app_for_test(AppConfig::for_test())
            .oneshot(
                Request::post("/session/bootstrap")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.starts_with(&format!("{SESSION_COOKIE}=")));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
    }

    #[tokio::test]
    async fn hunter_lab_demo_sets_fixed_session_cookie_and_redirects_home() {
        let config = AppConfig::for_test();
        let ttl = Duration::from_secs(config.session.ttl_seconds);
        let repository = Arc::new(InMemoryPlayerRepository::default());
        let coordinator = Arc::new(InMemorySessionCoordinator::default());
        let state = AppState {
            config: Arc::new(config),
            repository,
            coordinator,
            building_content: crate::simulation::test_authoritative_building_content(),
        };
        let response = crate::api::router(state.clone())
            .oneshot(
                Request::get("/session/demo/hunter-lab")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(response.headers().get(header::LOCATION).unwrap(), "/");
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cookie.starts_with(&format!(
            "{SESSION_COOKIE}={HUNTER_LAB_DEMO_SESSION_TOKEN};"
        )));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(resolve_player(&state, HUNTER_LAB_DEMO_SESSION_TOKEN, ttl)
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn durable_identity_recovers_a_cold_session_cache() {
        let config = AppConfig::for_test();
        let repository = Arc::new(InMemoryPlayerRepository::default());
        let token = Uuid::new_v4();
        let token_hash = SessionTokenHash::from_token(token);
        let player = repository
            .resolve_or_create_local_identity(token_hash)
            .await
            .unwrap();
        let state = AppState {
            config: Arc::new(config),
            repository,
            coordinator: Arc::new(InMemorySessionCoordinator::default()),
            building_content: crate::simulation::test_authoritative_building_content(),
        };
        let ttl = Duration::from_secs(60);

        assert_eq!(
            resolve_player(&state, token, ttl).await.unwrap(),
            Some(player)
        );
        assert_eq!(
            state.coordinator.resolve(token_hash, ttl).await.unwrap(),
            Some(player)
        );
    }
}
