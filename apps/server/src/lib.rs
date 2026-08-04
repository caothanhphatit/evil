pub mod api;
pub mod buildings;
pub mod config;
pub mod content;
pub mod coordination;
pub mod identity;
pub mod persistence;
pub mod simulation;

use std::sync::Arc;

use axum::Router;
use buildings::{
    AuthoritativeBuildingContent, BuildingRepository, BuildingRepositoryError,
    PostgresBuildingRepository,
};
use config::AppConfig;
use content::building_registry::EMBEDDED_REGISTRY_SHA256;
use coordination::{RedisSessionCoordinator, SharedSessionCoordinator};
use persistence::{PostgresPlayerRepository, SharedPlayerRepository};
use sqlx::{postgres::PgPoolOptions, PgPool};
use thiserror::Error;

const BUILDING_RELEASE_ID: &str = "evil-hunter-1.411.buildings-v1";
const HUNTER_PROFILE_RELEASE_ID: &str = "migration.hunter-demo-v1";
const HUNTER_INFO_RELEASE_ID: &str = "evil-hunter-1.411.hunter-info-v1";

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub repository: SharedPlayerRepository,
    pub coordinator: SharedSessionCoordinator,
    pub building_content: Arc<AuthoritativeBuildingContent>,
    pub admin_pool: Option<PgPool>,
}

#[derive(Debug, Error)]
pub enum AppBuildError {
    #[error("DATABASE_URL is required for the production server")]
    MissingDatabaseUrl,
    #[error("REDIS_URL is required for the production server")]
    MissingRedisUrl,
    #[error("invalid PostgreSQL configuration")]
    Postgres(#[from] sqlx::Error),
    #[error("invalid Redis configuration")]
    Redis(#[from] redis::RedisError),
    #[error("active building catalog is unavailable or does not match the pinned release")]
    BuildingCatalog(#[from] BuildingRepositoryError),
}

pub async fn app(config: AppConfig) -> Result<Router, AppBuildError> {
    let database_url = config
        .database_url
        .as_deref()
        .ok_or(AppBuildError::MissingDatabaseUrl)?;
    let redis_url = config
        .redis_url
        .as_deref()
        .ok_or(AppBuildError::MissingRedisUrl)?;
    let repository = Arc::new(PostgresPlayerRepository::connect_lazy(database_url)?);
    let admin_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(database_url)?;
    let building_repository = PostgresBuildingRepository::connect_lazy(database_url)?;
    let catalog = building_repository
        .load_catalog(BUILDING_RELEASE_ID, EMBEDDED_REGISTRY_SHA256)
        .await?;
    let gameplay = building_repository
        .load_gameplay_catalog(BUILDING_RELEASE_ID, EMBEDDED_REGISTRY_SHA256)
        .await?;
    let world_maps = building_repository
        .load_world_maps(BUILDING_RELEASE_ID)
        .await?;
    simulation::install_map_configs(world_maps).map_err(|_| {
        BuildingRepositoryError::InvalidCatalog("world map catalog installation failed")
    })?;
    let monster_pools = building_repository
        .load_ordinary_monster_pools(BUILDING_RELEASE_ID)
        .await?;
    simulation::install_monster_pools(monster_pools).map_err(|_| {
        BuildingRepositoryError::InvalidCatalog("ordinary monster catalog installation failed")
    })?;
    let progression = building_repository
        .load_hunter_progression(BUILDING_RELEASE_ID, simulation::EXPERIENCE_PROGRESSION_ID)
        .await?;
    simulation::install_experience_catalog(progression).map_err(|_| {
        BuildingRepositoryError::InvalidCatalog("Hunter progression installation failed")
    })?;
    let hunter_content = building_repository
        .load_hunter_static_content(HUNTER_PROFILE_RELEASE_ID, HUNTER_INFO_RELEASE_ID)
        .await?;
    simulation::install_hunter_static_content(hunter_content).map_err(|_| {
        BuildingRepositoryError::InvalidCatalog("Hunter static content installation failed")
    })?;
    let building_content = Arc::new(AuthoritativeBuildingContent::new(catalog, gameplay)?);
    let coordinator = Arc::new(RedisSessionCoordinator::new(redis_url)?);
    Ok(app_with_adapters(
        config,
        repository,
        coordinator,
        building_content,
        Some(admin_pool),
    ))
}

fn app_with_adapters(
    config: AppConfig,
    repository: SharedPlayerRepository,
    coordinator: SharedSessionCoordinator,
    building_content: Arc<AuthoritativeBuildingContent>,
    admin_pool: Option<PgPool>,
) -> Router {
    api::router(AppState {
        config: Arc::new(config),
        repository,
        coordinator,
        building_content,
        admin_pool,
    })
}

#[cfg(test)]
pub fn app_for_test(config: AppConfig) -> Router {
    use coordination::InMemorySessionCoordinator;
    use persistence::InMemoryPlayerRepository;

    app_with_adapters(
        config,
        Arc::new(InMemoryPlayerRepository::default()),
        Arc::new(InMemorySessionCoordinator::default()),
        crate::simulation::test_authoritative_building_content(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn production_app_requires_postgres_configuration() {
        let mut config = AppConfig::for_test();
        config.redis_url = Some("redis://localhost:6379/0".into());

        assert!(matches!(
            app(config).await,
            Err(AppBuildError::MissingDatabaseUrl)
        ));
    }

    #[tokio::test]
    async fn production_app_requires_redis_configuration() {
        let mut config = AppConfig::for_test();
        config.database_url = Some("postgres://localhost/evil_hunter".into());

        assert!(matches!(
            app(config).await,
            Err(AppBuildError::MissingRedisUrl)
        ));
    }
}
