use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    extract::{Extension, State},
    http::{header, HeaderMap, Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine as _,
};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{config::AdminRole, AppState};

const CSRF_TTL_SECONDS: u64 = 600;
const CSRF_HEADER: &str = "x-csrf-token";

#[derive(Clone, Debug)]
pub(super) struct AdminPrincipal {
    pub username: String,
    pub role: AdminRole,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CsrfResponse {
    token: String,
    expires_at: u64,
}

pub(super) async fn csrf_token(
    State(state): State<AppState>,
    Extension(principal): Extension<AdminPrincipal>,
) -> Response {
    let expires_at = unix_seconds().saturating_add(CSRF_TTL_SECONDS);
    let nonce = Uuid::new_v4();
    let token = sign_csrf_token(&state, &principal, expires_at, nonce);
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(CsrfResponse { token, expires_at }),
    )
        .into_response()
}

pub(super) async fn guard(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let Some(principal) = authenticate(&state, request.headers()) else {
        return unauthorized();
    };
    let rate_key = format!("actor:{}", principal.username);
    let rate_allowed = state
        .coordinator
        .allow_rate_limit(
            &rate_key,
            state.config.admin.rate_limit,
            Duration::from_millis(state.config.admin.rate_window_ms),
        )
        .await;
    match rate_allowed {
        Ok(true) => {}
        Ok(false) => {
            let retry_after = state
                .config
                .admin
                .rate_window_ms
                .div_ceil(1_000)
                .to_string();
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, retry_after)],
                Json(serde_json::json!({"error": "admin_rate_limited"})),
            )
                .into_response();
        }
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "admin_rate_limit_unavailable"})),
            )
                .into_response();
        }
    }

    let mutation = is_mutation(request.method());
    if mutation && !principal.role.permits_mutation() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "admin_role_forbidden"})),
        )
            .into_response();
    }
    if mutation && !valid_csrf_header(&state, &principal, request.headers()) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "admin_csrf_required"})),
        )
            .into_response();
    }

    let method = request.method().clone();
    // Axum strips the nest prefix before route middleware runs; audit the public path.
    let path = format!("/admin{}", request.uri().path());
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    request.extensions_mut().insert(principal.clone());
    let audit_id = if mutation {
        match begin_mutation_audit(&state, &principal, &method, &path, request_id.as_deref()).await
        {
            Some(audit_id) => Some(audit_id),
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error": "admin_audit_unavailable"})),
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    let response = next.run(request).await;
    if let Some(audit_id) = audit_id {
        complete_mutation_audit(&state, audit_id, response.status()).await;
    }
    response
}

pub(super) fn authenticate(state: &AppState, headers: &HeaderMap) -> Option<AdminPrincipal> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let encoded = value.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded).ok()?;
    let credentials = std::str::from_utf8(&decoded).ok()?;
    let (username, password) = credentials.split_once(':')?;
    (constant_time_digest_eq(username.as_bytes(), state.config.admin.username.as_bytes())
        & constant_time_digest_eq(password.as_bytes(), state.config.admin.password.as_bytes()))
    .then(|| AdminPrincipal {
        username: username.to_owned(),
        role: state.config.admin.role,
    })
}

pub(super) fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            "Basic realm=\"Evil Hunter Admin\"",
        )],
        Json(serde_json::json!({"error": "admin_auth_required"})),
    )
        .into_response()
}

fn is_mutation(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn sign_csrf_token(
    state: &AppState,
    principal: &AdminPrincipal,
    expires_at: u64,
    nonce: Uuid,
) -> String {
    let payload = format!("{expires_at}.{nonce}");
    let signature = csrf_signature(state, principal, &payload);
    format!("{payload}.{}", URL_SAFE_NO_PAD.encode(signature))
}

fn valid_csrf_header(state: &AppState, principal: &AdminPrincipal, headers: &HeaderMap) -> bool {
    let Some(token) = headers
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let mut parts = token.split('.');
    let (Some(expiry), Some(nonce), Some(signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let Ok(expires_at) = expiry.parse::<u64>() else {
        return false;
    };
    if expires_at < unix_seconds() || Uuid::parse_str(nonce).is_err() {
        return false;
    }
    let Ok(provided) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };
    let expected = csrf_signature(state, principal, &format!("{expiry}.{nonce}"));
    constant_time_eq(&provided, &expected)
}

fn csrf_signature(state: &AppState, principal: &AdminPrincipal, payload: &str) -> Vec<u8> {
    let secret = Sha256::digest(state.config.admin.password.as_bytes());
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&secret).expect("SHA-256 keys are valid HMAC keys");
    mac.update(principal.username.as_bytes());
    mac.update(b"\0");
    mac.update(principal.role.as_str().as_bytes());
    mac.update(b"\0");
    mac.update(payload.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

async fn begin_mutation_audit(
    state: &AppState,
    principal: &AdminPrincipal,
    method: &Method,
    path: &str,
    request_id: Option<&str>,
) -> Option<Uuid> {
    let Some(pool) = &state.admin_pool else {
        return None;
    };
    let result = sqlx::query_scalar(
        r#"INSERT INTO admin_mutation_audit
           (actor, role, method, path, request_id)
           VALUES ($1,$2,$3,$4,$5)
           RETURNING audit_id"#,
    )
    .bind(&principal.username)
    .bind(principal.role.as_str())
    .bind(method.as_str())
    .bind(path)
    .bind(request_id)
    .fetch_one(pool)
    .await;
    match result {
        Ok(audit_id) => Some(audit_id),
        Err(error) => {
            tracing::error!(%error, actor = principal.username, %path, "failed to begin admin mutation audit");
            None
        }
    }
}

async fn complete_mutation_audit(state: &AppState, audit_id: Uuid, status: StatusCode) {
    let Some(pool) = &state.admin_pool else {
        return;
    };
    if let Err(error) =
        sqlx::query("UPDATE admin_mutation_audit SET response_status = $2 WHERE audit_id = $1")
            .bind(audit_id)
            .bind(i32::from(status.as_u16()))
            .execute(pool)
            .await
    {
        tracing::error!(%error, %audit_id, "failed to complete admin mutation audit");
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn constant_time_digest_eq(left: &[u8], right: &[u8]) -> bool {
    constant_time_eq(&Sha256::digest(left), &Sha256::digest(right))
}
