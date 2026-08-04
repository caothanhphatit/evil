use std::{env, num::ParseIntError};

use axum::http::{header::InvalidHeaderValue, HeaderValue};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub web_origin: HeaderValue,
    pub log_format: String,
    pub database_url: Option<String>,
    pub redis_url: Option<String>,
    pub admin: AdminConfig,
    pub session: SessionConfig,
    pub simulation: SimulationConfig,
}

#[derive(Clone, Debug)]
pub struct AdminConfig {
    pub username: String,
    pub password: String,
    pub role: AdminRole,
    pub rate_limit: u32,
    pub rate_window_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminRole {
    Viewer,
    Operator,
    Admin,
}

impl AdminRole {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "viewer" => Some(Self::Viewer),
            "operator" => Some(Self::Operator),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
            Self::Admin => "admin",
        }
    }

    pub fn permits_mutation(self) -> bool {
        matches!(self, Self::Operator | Self::Admin)
    }
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub cookie_secure: bool,
    pub ttl_seconds: u64,
    pub lease_ttl_ms: u64,
    pub command_limit: u32,
    pub command_window_ms: u64,
}

#[derive(Clone, Debug)]
pub struct SimulationConfig {
    pub tick_rate: u32,
    pub seed: u64,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid integer in {name}: {source}")]
    InvalidInteger {
        name: &'static str,
        #[source]
        source: ParseIntError,
    },
    #[error("SIMULATION_TICK_RATE must be between 1 and 60")]
    InvalidTickRate,
    #[error("WEB_ORIGIN must be a valid HTTP Origin header: {0}")]
    InvalidWebOrigin(#[from] InvalidHeaderValue),
    #[error("session TTL, lease TTL, command limit, or command window is outside safe bounds")]
    InvalidSessionSettings,
    #[error("ADMIN_BASIC_AUTH_USER and ADMIN_BASIC_AUTH_PASSWORD must both be configured")]
    MissingAdminCredentials,
    #[error("ADMIN_BASIC_AUTH_ROLE must be viewer, operator, or admin")]
    InvalidAdminRole,
    #[error("admin rate limit or window is outside safe bounds")]
    InvalidAdminRateLimit,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let tick_rate = parse_env("SIMULATION_TICK_RATE", 10_u32)?;
        if !(1..=60).contains(&tick_rate) {
            return Err(ConfigError::InvalidTickRate);
        }

        let web_origin = HeaderValue::from_str(
            &env::var("WEB_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".into()),
        )?;

        let session = SessionConfig {
            cookie_secure: parse_bool_env("SESSION_COOKIE_SECURE", false),
            ttl_seconds: parse_env("SESSION_TTL_SECONDS", 604_800_u64)?,
            lease_ttl_ms: parse_env("PLAYER_LEASE_TTL_MS", 15_000_u64)?,
            command_limit: parse_env("COMMAND_RATE_LIMIT", 30_u32)?,
            command_window_ms: parse_env("COMMAND_RATE_WINDOW_MS", 1_000_u64)?,
        };
        if session.ttl_seconds < 60
            || session.lease_ttl_ms < 1_000
            || !(1..=1_000).contains(&session.command_limit)
            || !(100..=60_000).contains(&session.command_window_ms)
        {
            return Err(ConfigError::InvalidSessionSettings);
        }

        let admin = AdminConfig {
            username: non_empty_env("ADMIN_BASIC_AUTH_USER")
                .ok_or(ConfigError::MissingAdminCredentials)?,
            password: non_empty_env("ADMIN_BASIC_AUTH_PASSWORD")
                .ok_or(ConfigError::MissingAdminCredentials)?,
            role: AdminRole::parse(
                &env::var("ADMIN_BASIC_AUTH_ROLE").unwrap_or_else(|_| "admin".into()),
            )
            .ok_or(ConfigError::InvalidAdminRole)?,
            rate_limit: parse_env("ADMIN_RATE_LIMIT", 120_u32)?,
            rate_window_ms: parse_env("ADMIN_RATE_WINDOW_MS", 60_000_u64)?,
        };
        if !(10..=10_000).contains(&admin.rate_limit)
            || !(1_000..=3_600_000).contains(&admin.rate_window_ms)
        {
            return Err(ConfigError::InvalidAdminRateLimit);
        }

        Ok(Self {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: parse_env("SERVER_PORT", 8080_u16)?,
            web_origin,
            log_format: env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".into()),
            database_url: non_empty_env("DATABASE_URL"),
            redis_url: non_empty_env("REDIS_URL"),
            admin,
            session,
            simulation: SimulationConfig {
                tick_rate,
                seed: parse_env("SIMULATION_SEED", 6_840_227_782_638_526_189_u64)?,
            },
        })
    }

    pub fn for_test() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            web_origin: HeaderValue::from_static("http://localhost:5173"),
            log_format: "pretty".into(),
            database_url: None,
            redis_url: None,
            admin: AdminConfig {
                username: "admin".into(),
                password: "test-password".into(),
                role: AdminRole::Admin,
                rate_limit: 120,
                rate_window_ms: 60_000,
            },
            session: SessionConfig {
                cookie_secure: false,
                ttl_seconds: 604_800,
                lease_ttl_ms: 15_000,
                command_limit: 30,
                command_window_ms: 1_000,
            },
            simulation: SimulationConfig {
                tick_rate: 10,
                seed: 42,
            },
        }
    }
}

fn parse_bool_env(name: &'static str, default: bool) -> bool {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn non_empty_env(name: &'static str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn parse_env<T>(name: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr<Err = ParseIntError>,
{
    match env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|source| ConfigError::InvalidInteger { name, source }),
        Err(_) => Ok(default),
    }
}
