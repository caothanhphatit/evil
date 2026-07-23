use std::time::Duration;

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;

pub const SESSION_COOKIE: &str = "eh_session";

#[derive(Serialize)]
struct BootstrapResponse {
    status: &'static str,
}

pub async fn bootstrap(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let existing = session_token(&headers);
    let ttl = Duration::from_secs(state.config.session.ttl_seconds);
    let (token, _) = match state.coordinator.bootstrap(existing, ttl).await {
        Ok(session) => session,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let secure = if state.config.session.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}{}",
        state.config.session.ttl_seconds, secure
    );
    let mut response = Json(BootstrapResponse { status: "ready" }).into_response();
    if let Ok(value) = cookie.parse() {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
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

    use crate::{app, config::AppConfig};

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

    #[tokio::test]
    async fn bootstrap_sets_http_only_cookie_without_exposing_identity() {
        let response = app(AppConfig::for_test())
            .unwrap()
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
}
