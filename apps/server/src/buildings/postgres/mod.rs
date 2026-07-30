mod catalog;
mod numeric;
mod town;

use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};

const CATALOG_LOCK_KEY: &str = "evil_hunter_building_catalog";

#[derive(Clone)]
pub struct PostgresBuildingRepository {
    pool: PgPool,
}

impl PostgresBuildingRepository {
    pub fn connect_lazy(database_url: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect_lazy(database_url)?,
        })
    }

    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn lock_catalog(
        transaction: &mut Transaction<'_, Postgres>,
        shared: bool,
    ) -> Result<(), sqlx::Error> {
        let function = if shared {
            "pg_advisory_xact_lock_shared"
        } else {
            "pg_advisory_xact_lock"
        };
        let statement = format!("SELECT {function}(hashtext($1))");
        sqlx::query(&statement)
            .bind(CATALOG_LOCK_KEY)
            .execute(&mut **transaction)
            .await?;
        Ok(())
    }
}
