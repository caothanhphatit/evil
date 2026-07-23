pub mod api;
pub mod config;
pub mod coordination;
pub mod persistence;
pub mod simulation;

use std::sync::Arc;

use axum::Router;
use config::AppConfig;
use coordination::{InMemorySessionCoordinator, RedisSessionCoordinator, SharedSessionCoordinator};
use persistence::{InMemoryPlayerRepository, PostgresPlayerRepository, SharedPlayerRepository};
use thiserror::Error;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub repository: SharedPlayerRepository,
    pub coordinator: SharedSessionCoordinator,
}

#[derive(Debug, Error)]
pub enum AppBuildError {
    #[error("invalid PostgreSQL configuration")]
    Postgres(#[from] sqlx::Error),
    #[error("invalid Redis configuration")]
    Redis(#[from] redis::RedisError),
}

pub fn app(config: AppConfig) -> Result<Router, AppBuildError> {
    let repository: SharedPlayerRepository = match config.database_url.as_deref() {
        Some(database_url) => Arc::new(PostgresPlayerRepository::connect_lazy(database_url)?),
        None => Arc::new(InMemoryPlayerRepository::default()),
    };
    let coordinator: SharedSessionCoordinator = match config.redis_url.as_deref() {
        Some(redis_url) => Arc::new(RedisSessionCoordinator::new(redis_url)?),
        None => Arc::new(InMemorySessionCoordinator::default()),
    };
    Ok(api::router(AppState {
        config: Arc::new(config),
        repository,
        coordinator,
    }))
}
