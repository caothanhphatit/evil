use std::time::Duration;

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    coordination::CoordinationError, identity::SessionTokenHash, persistence::RepositoryError,
    AppState,
};

pub const SESSION_COOKIE: &str = "eh_session";
pub const HUNTER_LAB_DEMO_SESSION_TOKEN: Uuid = Uuid::from_u128(0x0000000000004000800000000000d001);
const PASSWORD_ITERATIONS: u32 = 20_000;

#[derive(Serialize)]
struct BootstrapResponse {
    status: &'static str,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    display_name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AccountResponse {
    status: &'static str,
    display_name: String,
    email: String,
    is_demo: bool,
}

#[derive(Debug, Error)]
pub enum SessionResolutionError {
    #[error("durable identity lookup failed")]
    Repository(#[from] RepositoryError),
    #[error("session cache failed")]
    Coordination(#[from] CoordinationError),
}

pub async fn bootstrap(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(token) = session_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let status = if resolve_player(
        &state,
        token,
        Duration::from_secs(state.config.session.ttl_seconds),
    )
    .await
    .ok()
    .flatten()
    .is_some()
    {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    };
    bootstrap_response(&state, token, status)
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Response {
    let display_name = body.display_name.trim();
    let email = normalize_email(&body.email);
    let display_name_length = display_name.chars().count();
    if !(2..=24).contains(&display_name_length)
        || email.len() > 254
        || !valid_email(&email)
        || body.password.len() < 8
        || body.password.len() > 128
    {
        return (StatusCode::BAD_REQUEST, "invalid_account_fields").into_response();
    }
    let password = body.password;
    let salt = *Uuid::new_v4().as_bytes();
    let password_hash =
        match tokio::task::spawn_blocking(move || hash_password(&password, &salt)).await {
            Ok(hash) => hash,
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
    let account = match state
        .repository
        .create_account(&email, display_name, &password_hash)
        .await
    {
        Ok(account) => account,
        Err(RepositoryError::AccountExists) => {
            return (StatusCode::CONFLICT, "account_exists").into_response()
        }
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    establish_account_session(&state, account, StatusCode::CREATED).await
}

pub async fn login(State(state): State<AppState>, Json(body): Json<LoginRequest>) -> Response {
    let email = normalize_email(&body.email);
    let account = match state.repository.find_account_by_email(&email).await {
        Ok(Some(account)) => account,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "invalid_credentials").into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let password = body.password;
    let password_hash = account.password_hash.clone();
    if !tokio::task::spawn_blocking(move || verify_password(&password, &password_hash))
        .await
        .unwrap_or(false)
    {
        return (StatusCode::UNAUTHORIZED, "invalid_credentials").into_response();
    }
    establish_account_session(&state, account, StatusCode::OK).await
}

async fn establish_account_session(
    state: &AppState,
    account: crate::persistence::PlayerAccountRecord,
    status: StatusCode,
) -> Response {
    let token = Uuid::new_v4();
    let token_hash = SessionTokenHash::from_token(token);
    if state
        .repository
        .bind_session(token_hash, account.player_token)
        .await
        .is_err()
        || state
            .coordinator
            .cache_session(
                token_hash,
                account.player_token,
                Duration::from_secs(state.config.session.ttl_seconds),
            )
            .await
            .is_err()
    {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let mut response = (
        status,
        Json(AccountResponse {
            status: "ready",
            display_name: account.display_name,
            email: account.normalized_email,
            is_demo: account.is_demo,
        }),
    )
        .into_response();
    set_session_cookie(state, token, &mut response);
    response
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

fn normalize_email(value: &str) -> String {
    value.trim().to_lowercase()
}

fn valid_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn hash_password(password: &str, salt: &[u8]) -> String {
    let iterations = PASSWORD_ITERATIONS;
    let derived = pbkdf2_sha256(password.as_bytes(), salt, iterations);
    format!(
        "$pbkdf2-sha256${iterations}${}${}",
        encode_hex(salt),
        encode_hex(&derived)
    )
}

fn verify_password(password: &str, encoded: &str) -> bool {
    let parts = encoded.split('$').collect::<Vec<_>>();
    if parts.len() != 5 || parts[1] != "pbkdf2-sha256" {
        return false;
    }
    let Ok(iterations) = parts[2].parse::<u32>() else {
        return false;
    };
    if iterations != PASSWORD_ITERATIONS {
        return false;
    }
    let Some(salt) = decode_hex(parts[3]) else {
        return false;
    };
    let Some(expected) = decode_hex(parts[4]) else {
        return false;
    };
    constant_time_eq(
        &pbkdf2_sha256(password.as_bytes(), &salt, iterations),
        &expected,
    )
}

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut result = [0_u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password, salt, iterations, &mut result);
    result
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.len() % 2 != 0 {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
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
    async fn bootstrap_requires_an_authenticated_account_cookie() {
        let response = app_for_test(AppConfig::for_test())
            .oneshot(
                Request::post("/session/bootstrap")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }

    #[test]
    fn password_hash_round_trips_and_rejects_wrong_password() {
        let encoded = hash_password("a-safe-password", b"0123456789abcdef");
        assert!(verify_password("a-safe-password", &encoded));
        assert!(!verify_password("wrong-password", &encoded));
        assert!(verify_password(
            "Demo1234!",
            "$pbkdf2-sha256$20000$6576696c2d68756e7465722d64656d6f2d31$93f3141640069161239cf63bd5b771040720a2127f35e746fb7e6b04e7090283"
        ));
    }

    #[tokio::test]
    async fn registering_then_logging_in_from_another_browser_resolves_same_player() {
        let repository = Arc::new(InMemoryPlayerRepository::default());
        let state = AppState {
            config: Arc::new(AppConfig::for_test()),
            repository: repository.clone(),
            coordinator: Arc::new(InMemorySessionCoordinator::default()),
            building_content: crate::simulation::test_authoritative_building_content(),
        };
        let router = crate::api::router(state);
        let register_response = router
            .clone()
            .oneshot(
                Request::post("/account/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"display_name":"Rin","email":"RIN@example.test","password":"password-123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(register_response.status(), StatusCode::CREATED);
        let first_cookie = register_response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let login_response = router
            .oneshot(
                Request::post("/account/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"email":"rin@example.test","password":"password-123"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login_response.status(), StatusCode::OK);
        let second_cookie = login_response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert_ne!(first_cookie, second_cookie);
        let account = repository
            .find_account_by_email("rin@example.test")
            .await
            .unwrap()
            .unwrap();
        assert!(!account.is_demo);
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
