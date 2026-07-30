use async_trait::async_trait;
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use super::{
    numeric::{optional_skin_id, to_u16, to_u32, to_u64},
    PostgresBuildingRepository,
};
use crate::buildings::{
    BaseBuildingId, BuildingRepositoryError, LoadedTownBuildingState, TownBuildingInstance,
    TownBuildingInstanceId, TownBuildingRepository, TownBuildingState, TownMaterialStock,
    TownProductStock, TownTradeSettlement,
};

#[async_trait]
impl TownBuildingRepository for PostgresBuildingRepository {
    async fn load_town(
        &self,
        player_token: Uuid,
    ) -> Result<Option<LoadedTownBuildingState>, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let loaded = self.load_town_in(&mut transaction, player_token).await?;
        transaction.commit().await?;
        Ok(loaded)
    }

    async fn save_town(
        &self,
        player_token: Uuid,
        state: &TownBuildingState,
        expected_revision: i64,
    ) -> Result<i64, BuildingRepositoryError> {
        let mut transaction = self.pool.begin().await?;
        let revision = self
            .save_town_in(&mut transaction, player_token, state, expected_revision)
            .await?;
        transaction.commit().await?;
        Ok(revision)
    }
}

impl PostgresBuildingRepository {
    pub(crate) async fn load_town_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        player_token: Uuid,
    ) -> Result<Option<LoadedTownBuildingState>, BuildingRepositoryError> {
        let row = sqlx::query(
            "SELECT town_id, release_id, gold, seed_version::bigint AS seed_version, \
                    next_building_sequence, revision \
             FROM town WHERE player_token = $1 FOR SHARE",
        )
        .bind(player_token)
        .fetch_optional(&mut **transaction)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let town_id: Uuid = row.try_get("town_id")?;
        let rows = sqlx::query(
            r#"SELECT instance_id, building_id, equipped_skin_id,
                      current_level::bigint AS current_level,
                      grid_x, grid_y, use_count, seeded_by
               FROM player_building WHERE town_id = $1 ORDER BY instance_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let mut buildings = Vec::with_capacity(rows.len());
        for building in rows {
            buildings.push(TownBuildingInstance {
                instance_id: TownBuildingInstanceId::new(building.try_get("instance_id")?),
                building_id: BaseBuildingId::parse(building.try_get::<String, _>("building_id")?)?,
                equipped_skin_id: optional_skin_id(building.try_get("equipped_skin_id")?)?,
                level: to_u16(building.try_get("current_level")?)?,
                uses: to_u32(building.try_get("use_count")?)?,
                grid_x: building.try_get("grid_x")?,
                grid_y: building.try_get("grid_y")?,
                seeded_by: building.try_get("seeded_by")?,
            });
        }
        let economy = sqlx::query(
            r#"SELECT hunter_materials, materials, runes, weapons, armor,
                      hunter_equipment_purchases
               FROM town_economy_summary WHERE town_id = $1"#,
        )
        .bind(town_id)
        .fetch_one(&mut **transaction)
        .await?;
        let trade_state = sqlx::query(
            "SELECT field_trip_id, settled_field_trip_id FROM town_trade_state WHERE town_id = $1",
        )
        .bind(town_id)
        .fetch_one(&mut **transaction)
        .await?;
        let stock_rows = sqlx::query(
            r#"SELECT inventory.item_id,
                      inventory.quantity AS town_quantity,
                      COALESCE(hunter.quantity, 0) AS hunter_quantity,
                      COALESCE(orders.requested_quantity - orders.fulfilled_quantity, 0) AS requested,
                      COALESCE(orders.unit_price, 0) AS unit_price
               FROM town_inventory_stack AS inventory
               LEFT JOIN hunter_material_stack AS hunter
                 ON hunter.town_id = inventory.town_id
                AND hunter.material_id = inventory.item_id
               LEFT JOIN building_material_order AS orders
                 ON orders.town_id = inventory.town_id
                AND orders.material_id = inventory.item_id
                AND orders.status = 'open'
               WHERE inventory.town_id = $1
               UNION
               SELECT hunter.material_id, 0, hunter.quantity, 0, 0
               FROM hunter_material_stack AS hunter
               WHERE hunter.town_id = $1
                 AND NOT EXISTS (
                     SELECT 1 FROM town_inventory_stack AS inventory
                     WHERE inventory.town_id = hunter.town_id
                       AND inventory.item_id = hunter.material_id
                 )
               UNION
               SELECT orders.material_id, 0, 0,
                      orders.requested_quantity - orders.fulfilled_quantity, orders.unit_price
               FROM building_material_order AS orders
               WHERE orders.town_id = $1 AND orders.status = 'open'
                 AND NOT EXISTS (
                     SELECT 1 FROM town_inventory_stack AS inventory
                     WHERE inventory.town_id = orders.town_id
                       AND inventory.item_id = orders.material_id
                 )
                 AND NOT EXISTS (
                     SELECT 1 FROM hunter_material_stack AS hunter
                     WHERE hunter.town_id = orders.town_id
                       AND hunter.material_id = orders.material_id
                 )
               ORDER BY item_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let material_stocks = stock_rows
            .into_iter()
            .map(|stock| {
                Ok(TownMaterialStock {
                    id: stock.try_get("item_id")?,
                    town_quantity: to_u32(stock.try_get("town_quantity")?)?,
                    hunter_quantity: to_u32(stock.try_get("hunter_quantity")?)?,
                    requested: to_u32(stock.try_get("requested")?)?,
                    unit_price: to_u64(stock.try_get("unit_price")?)?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let product_stock_rows = sqlx::query(
            r#"SELECT building_instance_id, product_id, quantity
               FROM building_product_stock
               WHERE town_id = $1
               ORDER BY building_instance_id, product_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let product_stocks = product_stock_rows
            .into_iter()
            .map(|stock| {
                Ok(TownProductStock {
                    building_instance_id: TownBuildingInstanceId::new(
                        stock.try_get("building_instance_id")?,
                    ),
                    product_id: stock.try_get("product_id")?,
                    quantity: to_u32(stock.try_get("quantity")?)?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let settlement_rows = sqlx::query(
            r#"SELECT settlement_id, field_trip_id, material_id, quantity,
                      unit_price, total_gold
               FROM hunter_trade_settlement
               WHERE town_id = $1 ORDER BY settled_at, settlement_id"#,
        )
        .bind(town_id)
        .fetch_all(&mut **transaction)
        .await?;
        let trade_settlements = settlement_rows
            .into_iter()
            .map(|settlement| {
                Ok(TownTradeSettlement {
                    settlement_id: settlement.try_get("settlement_id")?,
                    field_trip_id: to_u64(settlement.try_get("field_trip_id")?)?,
                    material_id: settlement.try_get("material_id")?,
                    quantity: to_u32(settlement.try_get("quantity")?)?,
                    unit_price: to_u64(settlement.try_get("unit_price")?)?,
                    total_gold: to_u64(settlement.try_get("total_gold")?)?,
                })
            })
            .collect::<Result<Vec<_>, BuildingRepositoryError>>()?;
        let loaded = LoadedTownBuildingState {
            state: TownBuildingState {
                release_id: row.try_get("release_id")?,
                town_gold: to_u64(row.try_get("gold")?)?,
                seed_version: to_u16(row.try_get("seed_version")?)?,
                next_building_sequence: to_u64(row.try_get("next_building_sequence")?)?,
                buildings,
                hunter_materials: to_u32(economy.try_get("hunter_materials")?)?,
                materials: to_u32(economy.try_get("materials")?)?,
                runes: to_u32(economy.try_get("runes")?)?,
                weapons: to_u32(economy.try_get("weapons")?)?,
                armor: to_u32(economy.try_get("armor")?)?,
                hunter_equipment_purchases: to_u32(economy.try_get("hunter_equipment_purchases")?)?,
                field_trip_id: to_u64(trade_state.try_get("field_trip_id")?)?,
                settled_field_trip_id: to_u64(trade_state.try_get("settled_field_trip_id")?)?,
                material_stocks,
                product_stocks,
                trade_settlements,
            },
            revision: row.try_get("revision")?,
        };
        loaded.state.validate()?;
        Ok(Some(loaded))
    }

    pub(crate) async fn create_town_from_default_template_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        player_token: Uuid,
        release_id: &str,
        initial_gold: u64,
    ) -> Result<LoadedTownBuildingState, BuildingRepositoryError> {
        let town_row = sqlx::query(
            r#"INSERT INTO town (
                   player_token, release_id, source_template_id, gold,
                   next_building_sequence, revision
               )
               SELECT $1, template.release_id, template.template_id, $3,
                      count(template_building.slot) + 1, 0
               FROM town_template AS template
               LEFT JOIN town_template_building AS template_building
                 ON template_building.template_id = template.template_id
                AND template_building.release_id = template.release_id
               WHERE template.release_id = $2 AND template.is_default
               GROUP BY template.template_id, template.release_id
               ON CONFLICT (player_token) DO NOTHING
               RETURNING town_id"#,
        )
        .bind(player_token)
        .bind(release_id)
        .bind(i64::try_from(initial_gold).map_err(|_| BuildingRepositoryError::NumericBounds)?)
        .fetch_optional(&mut **transaction)
        .await?;

        if let Some(town_row) = town_row {
            let town_id: Uuid = town_row.try_get("town_id")?;
            sqlx::query(
                "INSERT INTO town_economy_summary (town_id, hunter_materials) VALUES ($1, 20)",
            )
            .bind(town_id)
            .execute(&mut **transaction)
            .await?;
            sqlx::query("INSERT INTO town_trade_state (town_id) VALUES ($1)")
                .bind(town_id)
                .execute(&mut **transaction)
                .await?;
            sqlx::query(
                r#"INSERT INTO player_building (
                       town_id, release_id, building_id, current_level,
                       equipped_skin_id, grid_x, grid_y, use_count, seeded_by
                   )
                   SELECT $1, template.release_id, template.building_id, template.level,
                          template.equipped_skin_id, template.grid_x, template.grid_y,
                          0, source.template_id
                   FROM town_template_building AS template
                   JOIN town_template AS source
                     ON source.template_id = template.template_id
                    AND source.release_id = template.release_id
                   WHERE template.release_id = $2 AND source.is_default
                   ORDER BY template.slot"#,
            )
            .bind(town_id)
            .bind(release_id)
            .execute(&mut **transaction)
            .await?;
        }

        self.load_town_in(transaction, player_token).await?.ok_or(
            BuildingRepositoryError::InvalidTown("active release has no default town template"),
        )
    }

    pub(crate) async fn save_town_in(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        player_token: Uuid,
        state: &TownBuildingState,
        expected_revision: i64,
    ) -> Result<i64, BuildingRepositoryError> {
        state.validate()?;
        let town_row = sqlx::query(
            r#"UPDATE town SET
                   gold = $2,
                   seed_version = $3,
                   next_building_sequence = $4,
                   revision = town.revision + 1,
                   updated_at = now()
               WHERE player_token = $1 AND release_id = $5 AND revision = $6
               RETURNING town_id, revision"#,
        )
        .bind(player_token)
        .bind(i64::try_from(state.town_gold).map_err(|_| BuildingRepositoryError::NumericBounds)?)
        .bind(i64::from(state.seed_version))
        .bind(
            i64::try_from(state.next_building_sequence)
                .map_err(|_| BuildingRepositoryError::NumericBounds)?,
        )
        .bind(&state.release_id)
        .bind(expected_revision)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(BuildingRepositoryError::RevisionConflict)?;
        let town_id: Uuid = town_row.try_get("town_id")?;
        let revision: i64 = town_row.try_get("revision")?;

        sqlx::query(
            r#"INSERT INTO town_economy_summary (
                   town_id, hunter_materials, materials, runes, weapons, armor,
                   hunter_equipment_purchases
               ) VALUES ($1,$2,$3,$4,$5,$6,$7)
               ON CONFLICT (town_id) DO UPDATE SET
                   hunter_materials = EXCLUDED.hunter_materials,
                   materials = EXCLUDED.materials,
                   runes = EXCLUDED.runes,
                   weapons = EXCLUDED.weapons,
                   armor = EXCLUDED.armor,
                   hunter_equipment_purchases = EXCLUDED.hunter_equipment_purchases,
                   updated_at = now()"#,
        )
        .bind(town_id)
        .bind(i64::from(state.hunter_materials))
        .bind(i64::from(state.materials))
        .bind(i64::from(state.runes))
        .bind(i64::from(state.weapons))
        .bind(i64::from(state.armor))
        .bind(i64::from(state.hunter_equipment_purchases))
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            r#"INSERT INTO town_trade_state (town_id, field_trip_id, settled_field_trip_id)
               VALUES ($1,$2,$3)
               ON CONFLICT (town_id) DO UPDATE SET
                   field_trip_id = EXCLUDED.field_trip_id,
                   settled_field_trip_id = EXCLUDED.settled_field_trip_id,
                   updated_at = now()"#,
        )
        .bind(town_id)
        .bind(
            i64::try_from(state.field_trip_id)
                .map_err(|_| BuildingRepositoryError::NumericBounds)?,
        )
        .bind(
            i64::try_from(state.settled_field_trip_id)
                .map_err(|_| BuildingRepositoryError::NumericBounds)?,
        )
        .execute(&mut **transaction)
        .await?;

        sqlx::query(
            "UPDATE town_inventory_stack SET quantity = 0, updated_at = now() WHERE town_id = $1",
        )
        .bind(town_id)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "UPDATE hunter_material_stack SET quantity = 0, updated_at = now() WHERE town_id = $1",
        )
        .bind(town_id)
        .execute(&mut **transaction)
        .await?;
        let mut requested_material_ids = Vec::new();
        for stock in &state.material_stocks {
            sqlx::query(
                r#"INSERT INTO town_inventory_stack (town_id, item_id, quantity)
                   VALUES ($1,$2,$3)
                   ON CONFLICT (town_id, item_id) DO UPDATE SET
                       quantity = EXCLUDED.quantity, updated_at = now()"#,
            )
            .bind(town_id)
            .bind(&stock.id)
            .bind(i64::from(stock.town_quantity))
            .execute(&mut **transaction)
            .await?;
            sqlx::query(
                r#"INSERT INTO hunter_material_stack (town_id, material_id, quantity)
                   VALUES ($1,$2,$3)
                   ON CONFLICT (town_id, material_id) DO UPDATE SET
                       quantity = EXCLUDED.quantity, updated_at = now()"#,
            )
            .bind(town_id)
            .bind(&stock.id)
            .bind(i64::from(stock.hunter_quantity))
            .execute(&mut **transaction)
            .await?;
            if stock.requested > 0 {
                requested_material_ids.push(stock.id.clone());
                sqlx::query(
                    r#"INSERT INTO building_material_order (
                           town_id, material_id, requested_quantity,
                           fulfilled_quantity, unit_price, status
                       ) VALUES ($1,$2,$3,0,$4,'open')
                       ON CONFLICT (town_id, material_id) WHERE status = 'open'
                       DO UPDATE SET
                           requested_quantity = EXCLUDED.requested_quantity,
                           fulfilled_quantity = 0,
                           unit_price = EXCLUDED.unit_price,
                           updated_at = now()"#,
                )
                .bind(town_id)
                .bind(&stock.id)
                .bind(i64::from(stock.requested))
                .bind(
                    i64::try_from(stock.unit_price)
                        .map_err(|_| BuildingRepositoryError::NumericBounds)?,
                )
                .execute(&mut **transaction)
                .await?;
            }
        }
        sqlx::query(
            "UPDATE building_material_order SET status = 'cancelled', updated_at = now() \
             WHERE town_id = $1 AND status = 'open' AND NOT (material_id = ANY($2))",
        )
        .bind(town_id)
        .bind(&requested_material_ids)
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "UPDATE building_product_stock SET quantity = 0, updated_at = now() WHERE town_id = $1",
        )
        .bind(town_id)
        .execute(&mut **transaction)
        .await?;
        for stock in &state.product_stocks {
            sqlx::query(
                r#"INSERT INTO building_product_stock (
                       town_id, building_instance_id, release_id, building_id,
                       product_id, quantity
                   )
                   SELECT $1, building.instance_id, building.release_id,
                          building.building_id, $3, $4
                   FROM player_building AS building
                   WHERE building.town_id = $1 AND building.instance_id = $2
                   ON CONFLICT (town_id, building_instance_id, product_id) DO UPDATE SET
                       quantity = EXCLUDED.quantity, updated_at = now()"#,
            )
            .bind(town_id)
            .bind(stock.building_instance_id.get())
            .bind(&stock.product_id)
            .bind(i64::from(stock.quantity))
            .execute(&mut **transaction)
            .await?;
        }
        for settlement in &state.trade_settlements {
            let result = sqlx::query(
                r#"INSERT INTO hunter_trade_settlement (
                       town_id, settlement_id, field_trip_id, material_id,
                       quantity, unit_price, total_gold
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7)
                   ON CONFLICT (town_id, settlement_id) DO UPDATE SET
                       settlement_id = EXCLUDED.settlement_id
                   WHERE hunter_trade_settlement.field_trip_id = EXCLUDED.field_trip_id
                     AND hunter_trade_settlement.material_id = EXCLUDED.material_id
                     AND hunter_trade_settlement.quantity = EXCLUDED.quantity
                     AND hunter_trade_settlement.unit_price = EXCLUDED.unit_price
                     AND hunter_trade_settlement.total_gold = EXCLUDED.total_gold"#,
            )
            .bind(town_id)
            .bind(&settlement.settlement_id)
            .bind(
                i64::try_from(settlement.field_trip_id)
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .bind(&settlement.material_id)
            .bind(i64::from(settlement.quantity))
            .bind(
                i64::try_from(settlement.unit_price)
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .bind(
                i64::try_from(settlement.total_gold)
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .execute(&mut **transaction)
            .await?;
            if result.rows_affected() != 1 {
                return Err(BuildingRepositoryError::InvalidTown(
                    "trade settlement identity conflicts with persisted data",
                ));
            }
        }

        let mut retained_ids = Vec::with_capacity(state.buildings.len());
        for building in &state.buildings {
            retained_ids.push(building.instance_id.get());
            sqlx::query(
                r#"INSERT INTO player_building
                       (instance_id, town_id, release_id, building_id, current_level,
                        equipped_skin_id, grid_x, grid_y, use_count, seeded_by)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                   ON CONFLICT (instance_id) DO UPDATE SET
                       current_level = EXCLUDED.current_level,
                       equipped_skin_id = EXCLUDED.equipped_skin_id,
                       grid_x = EXCLUDED.grid_x,
                       grid_y = EXCLUDED.grid_y,
                       use_count = EXCLUDED.use_count,
                       seeded_by = EXCLUDED.seeded_by,
                       updated_at = now()
                   WHERE player_building.town_id = EXCLUDED.town_id
                     AND player_building.release_id = EXCLUDED.release_id
                     AND player_building.building_id = EXCLUDED.building_id"#,
            )
            .bind(building.instance_id.get())
            .bind(town_id)
            .bind(&state.release_id)
            .bind(building.building_id.as_str())
            .bind(i64::from(building.level))
            .bind(
                building
                    .equipped_skin_id
                    .map(|skin| i64::try_from(skin.get()))
                    .transpose()
                    .map_err(|_| BuildingRepositoryError::NumericBounds)?,
            )
            .bind(building.grid_x)
            .bind(building.grid_y)
            .bind(i64::from(building.uses))
            .bind(&building.seeded_by)
            .execute(&mut **transaction)
            .await?;
        }
        sqlx::query(
            "DELETE FROM player_building WHERE town_id = $1 AND NOT (instance_id = ANY($2))",
        )
        .bind(town_id)
        .bind(&retained_ids)
        .execute(&mut **transaction)
        .await?;
        Ok(revision)
    }
}
